#!/bin/bash
# Build all Windows-optimized coreutils

echo "🔨 Building Windows-optimized coreutils..."

# Build the winpath module first
echo "📦 Building winpath module..."
cd winpath && cargo build --release
cd ..

# Build all utilities
echo "📦 Building utilities..."
cargo build --release --bin tac
cargo build --release --bin cksum
cargo build --release --bin numfmt
cargo build --release --bin date
cargo build --release --bin cut
cargo build --release --bin true
cargo build --release --bin unlink
cargo build --release --bin dircolors
cargo build --release --bin tr
cargo build --release --bin seq
cargo build --release --bin sync
cargo build --release --bin rmdir
cargo build --release --bin du
cargo build --release --bin vdir
cargo build --release --bin dd
cargo build --release --bin uniq
cargo build --release --bin yes
cargo build --release --bin sort
cargo build --release --bin cat
cargo build --release --bin ptx
cargo build --release --bin base64
cargo build --release --bin realpath
cargo build --release --bin rm
cargo build --release --bin nl
cargo build --release --bin shuf
cargo build --release --bin mkdir
cargo build --release --bin split
cargo build --release --bin more
cargo build --release --bin echo
cargo build --release --bin shred
cargo build --release --bin readlink
cargo build --release --bin ln
cargo build --release --bin env
cargo build --release --bin fold
cargo build --release --bin hashsum
cargo build --release --bin truncate
cargo build --release --bin printf
cargo build --release --bin base32
cargo build --release --bin head
cargo build --release --bin fmt
cargo build --release --bin od
cargo build --release --bin test
cargo build --release --bin hostname
cargo build --release --bin link
cargo build --release --bin df
cargo build --release --bin false
cargo build --release --bin csplit
cargo build --release --bin whoami
cargo build --release --bin pwd
cargo build --release --bin comm
cargo build --release --bin dir
cargo build --release --bin basename
cargo build --release --bin mv
cargo build --release --bin factor
cargo build --release --bin nproc
cargo build --release --bin printenv
cargo build --release --bin tsort
cargo build --release --bin unexpand
cargo build --release --bin sleep
cargo build --release --bin tail
cargo build --release --bin basenc
cargo build --release --bin join
cargo build --release --bin arch
cargo build --release --bin mktemp
cargo build --release --bin wc
cargo build --release --bin dirname
cargo build --release --bin expr
cargo build --release --bin paste
cargo build --release --bin sum
cargo build --release --bin cp
cargo build --release --bin expand
cargo build --release --bin tee
cargo build --release --bin touch
cargo build --release --bin pr

echo "✅ Build complete! All utilities available in target/release/"
