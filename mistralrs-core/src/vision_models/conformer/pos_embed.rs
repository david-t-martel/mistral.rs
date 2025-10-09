#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]

use candle_core::{DType, Device, IndexOp, Result, Tensor, D};
use candle_nn::{Embedding, Module};
use mistralrs_quant::ShardedVarBuilder;

use crate::layers;

use super::config::ConformerEncoderConfig;

#[allow(unused)]
pub struct AbsolutePositionalEncoding {
    pe: Tensor,
    xscale: f64,
}

impl AbsolutePositionalEncoding {
    pub fn new(cfg: &ConformerEncoderConfig, device: &Device) -> Result<Self> {
        let max_len = 5000;

        let mut pe = Tensor::zeros((max_len, cfg.attention_dim), DType::F32, device)?;
        let position = Tensor::arange(0u32, max_len as u32, device)?.unsqueeze(1)?;

        let div_term = (Tensor::arange_step(0u32, cfg.attention_dim as u32, 2, device)?
            .to_dtype(DType::F32)?
            * -((10000f64).ln() / cfg.attention_dim as f64))?
            .exp()?;

        let sin = position
            .to_dtype(DType::F32)?
            .broadcast_mul(&div_term)?
            .sin()?;
        let cos = position
            .to_dtype(DType::F32)?
            .broadcast_mul(&div_term)?
            .cos()?;

        // Interleave
        let sin_indices = Tensor::from_vec(
            (0..cfg.attention_dim)
                .step_by(2)
                .map(|x| x as u32)
                .collect(),
            cfg.attention_dim / 2,
            device,
        )?;
        let cos_indices = Tensor::from_vec(
            (1..cfg.attention_dim)
                .step_by(2)
                .map(|x| x as u32)
                .collect(),
            cfg.attention_dim / 2,
            device,
        )?;
        pe = pe.index_add(&sin_indices, &sin, D::Minus1)?;
        pe = pe.index_add(&cos_indices, &cos, D::Minus1)?;
        pe = pe.unsqueeze(0)?;

        Ok(Self {
            pe,
            xscale: (cfg.attention_dim as f64).sqrt(),
        })
    }

    #[allow(unused)]
    pub fn forward(&self, xs: &Tensor) -> Result<Tensor> {
        if xs.dim(1)? >= self.pe.dim(1)? {
            candle_core::bail!("Need to recompute positional embeds");
        }

        (xs * self.xscale)?.broadcast_add(&self.pe.i((.., ..xs.dim(1)?))?.to_dtype(xs.dtype())?)
    }
}

pub struct T5RelativeAttentionLogitBias {
    bias_values: Embedding,
    skip_bucketing: bool,
    max_distance: usize,
    symmetric: bool,
}

impl T5RelativeAttentionLogitBias {
    pub fn new(
        num_heads: usize,
        num_buckets: Option<usize>,
        max_distance: usize,
        symmetric: bool,
        vb: ShardedVarBuilder,
    ) -> Result<Self> {
        let skip_bucketing = num_buckets.is_none();
        let mut num_buckets = num_buckets.unwrap_or(max_distance);
        if !symmetric {
            num_buckets *= 2;
        }

        Ok(Self {
            bias_values: layers::embedding(num_buckets, num_heads, vb.pp("bias_values"), &None)?,
            skip_bucketing,
            symmetric,
            max_distance,
        })
    }

    pub fn forward(&self, x: &Tensor) -> Result<Tensor> {
        let maxpos = x.dim(1)?;
        let device = x.device();

        // Create position matrices
        let context_position = Tensor::arange(0f32, maxpos as f32, device)?.unsqueeze(1)?;
        let memory_position = Tensor::arange(0f32, maxpos as f32, device)?.unsqueeze(0)?;

        // Calculate relative positions
        let relative_position = memory_position.broadcast_sub(&context_position)?;

        // Clip to max distance (equivalent to Python's masked_fill)
        let max_dist = self.max_distance as i64;
        let relative_position = relative_position.clamp(-max_dist, max_dist - 1)?;

        // Map to bias indices
        let bias_idx = if self.skip_bucketing {
            let idx = if self.symmetric {
                relative_position.abs()?
            } else {
                let offset = (self.bias_values.embeddings().dim(0)? / 2) as i64;
                (relative_position + offset as f64)?
            };
            idx.to_dtype(DType::U32)?
        } else {
            self.bucketize_relative_positions(&relative_position)?
        };

        // Get bias values
        let t5_rel_att_bias = self.bias_values.forward(&bias_idx)?; // [L, L, H]
        t5_rel_att_bias.permute((2, 0, 1))?.unsqueeze(0) // [1, H, L, L]
    }

    fn bucketize_relative_positions(&self, relative_position: &Tensor) -> Result<Tensor> {
        let device = relative_position.device();
        let dims = relative_position.dims();
        let total_buckets = self.bias_values.embeddings().dim(0)? as i64;
        if total_buckets <= 0 {
            candle_core::bail!("Relative attention bias must have at least one bucket.");
        }
        let max_distance = self.max_distance as i64;
        let positions = relative_position.flatten_all()?.to_vec1::<f32>()?;
        let mut buckets = Vec::with_capacity(positions.len());
        for raw in positions {
            let position = raw.round() as i64;
            let bucket = self.assign_bucket(position, total_buckets, max_distance);
            buckets.push(bucket as u32);
        }
        Tensor::from_vec(buckets, dims, device)
    }

    fn assign_bucket(&self, position: i64, total_buckets: i64, max_distance: i64) -> i64 {
        if total_buckets <= 1 {
            return 0;
        }
        let mut bucket = if self.symmetric {
            let half = (total_buckets / 2).max(1);
            let offset = if position > 0 { half } else { 0 };
            let abs_pos = position.abs();
            offset + self.bucket_single_side(abs_pos, half, max_distance)
        } else {
            let pos = if position < 0 { -position } else { 0 };
            self.bucket_single_side(pos, total_buckets, max_distance)
        };
        if bucket >= total_buckets {
            bucket = total_buckets - 1;
        }
        bucket.max(0)
    }

    fn bucket_single_side(&self, position: i64, num_buckets: i64, max_distance: i64) -> i64 {
        if num_buckets <= 1 {
            return 0;
        }
        let max_exact = (num_buckets / 2).max(1);
        if position < max_exact {
            return position;
        }
        let remaining = (num_buckets - max_exact).max(1);
        let log_ratio = ((position as f64) / (max_exact as f64)).ln();
        let base_denominator = ((max_distance.max(max_exact)) as f64 / max_exact as f64).ln();
        let mut bucket = max_exact;
        if base_denominator.is_finite() && base_denominator > 0.0 {
            bucket += ((log_ratio / base_denominator) * remaining as f64).floor() as i64;
        }
        bucket.min(num_buckets - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::{DType, Device, Tensor};

    fn make_bias(
        skip_bucketing: bool,
        symmetric: bool,
        total_buckets: usize,
        max_distance: usize,
        num_heads: usize,
    ) -> candle_core::Result<T5RelativeAttentionLogitBias> {
        let device = Device::Cpu;
        let weights = Tensor::zeros((total_buckets, num_heads), DType::F32, &device)?;
        let embedding = Embedding::new(weights, num_heads);
        Ok(T5RelativeAttentionLogitBias {
            bias_values: embedding,
            skip_bucketing,
            max_distance,
            symmetric,
        })
    }

    fn reference_single_side(position: i64, num_buckets: i64, max_distance: i64) -> i64 {
        if num_buckets <= 1 {
            return 0;
        }
        let max_exact = (num_buckets / 2).max(1);
        if position < max_exact {
            return position.min(num_buckets - 1);
        }
        let remaining = (num_buckets - max_exact).max(1);
        let log_denominator = ((max_distance.max(max_exact as i64)) as f64 / max_exact as f64).ln();
        let mut bucket = max_exact;
        if log_denominator.is_finite() && log_denominator > 0.0 {
            let ratio = (position as f64 / max_exact as f64).ln();
            bucket += ((ratio / log_denominator) * remaining as f64).floor() as i64;
        }
        bucket.min(num_buckets - 1)
    }

    fn reference_bucket(
        position: i64,
        total_buckets: i64,
        max_distance: i64,
        symmetric: bool,
    ) -> i64 {
        if symmetric {
            let half = (total_buckets / 2).max(1);
            let offset = if position > 0 { half } else { 0 };
            let abs_pos = position.abs();
            offset + reference_single_side(abs_pos, half, max_distance)
        } else {
            let pos = if position < 0 { -position } else { 0 };
            reference_single_side(pos, total_buckets, max_distance)
        }
    }

    #[test]
    fn bucket_assignment_matches_reference() -> candle_core::Result<()> {
        let bias = make_bias(false, true, 32, 128, 2)?;
        let positions = [-512, -20, -3, -1, 0, 1, 3, 20, 512];
        for pos in positions {
            let expect = reference_bucket(pos, 32, 128, true);
            let actual = bias.assign_bucket(pos, 32, 128);
            assert_eq!(actual, expect, "bucket mismatch at position {pos}");
        }
        Ok(())
    }

    #[test]
    fn bucketize_tensor_matches_scalar_assignment() -> candle_core::Result<()> {
        let bias = make_bias(false, true, 32, 128, 2)?;
        let raw_positions = vec![-64f32, -4.0, -1.0, 0.0, 1.0, 4.0, 64.0];
        let positions = Tensor::from_vec(
            raw_positions.clone(),
            (1, raw_positions.len()),
            &Device::Cpu,
        )?;
        let buckets = bias.bucketize_relative_positions(&positions)?;
        let values = buckets.flatten_all()?.to_vec1::<u32>()?;
        for (idx, bucket) in values.iter().enumerate() {
            let expect = reference_bucket(raw_positions[idx].round() as i64, 32, 128, true) as u32;
            assert_eq!(*bucket, expect, "bucket mismatch at index {idx}");
        }
        Ok(())
    }

    #[test]
    fn skip_bucketing_returns_absolute_offsets() -> candle_core::Result<()> {
        let bias = make_bias(true, true, 8, 4, 2)?;
        let positions = Tensor::from_vec(vec![-2f32, -1.0, 0.0, 1.0, 2.0], (1, 5), &Device::Cpu)?;
        let buckets = bias.bucketize_relative_positions(&positions)?;
        let values = buckets.flatten_all()?.to_vec1::<u32>()?;
        assert_eq!(values, vec![2, 1, 0, 5, 6]);
        Ok(())
    }
}
