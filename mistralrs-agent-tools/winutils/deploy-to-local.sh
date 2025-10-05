#!/bin/bash

# Deploy Windows coreutils to ~/.local/bin with canonical names
# Creates backups of existing executables in .archive

WINUTILS_DIR="/mnt/t/projects/coreutils/winutils"
TARGET_DIR="/mnt/c/users/david/.local/bin"
ARCHIVE_DIR="${TARGET_DIR}/.archive"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")

echo "Deploying Windows coreutils to ${TARGET_DIR}"
echo "Backup timestamp: ${TIMESTAMP}"
echo

# Create archive directory if needed
mkdir -p "${ARCHIVE_DIR}"

# List of all utilities with their binary names and canonical names
declare -A UTILITIES=(
    # Coreutils (74 utilities)
    ["uu_arch.exe"]="arch.exe"
    ["uu_base32.exe"]="base32.exe"
    ["uu_base64.exe"]="base64.exe"
    ["uu_basename.exe"]="basename.exe"
    ["uu_basenc.exe"]="basenc.exe"
    ["uu_cat.exe"]="cat.exe"
    ["uu_cksum.exe"]="cksum.exe"
    ["uu_comm.exe"]="comm.exe"
    ["uu_cp.exe"]="cp.exe"
    ["uu_csplit.exe"]="csplit.exe"
    ["uu_cut.exe"]="cut.exe"
    ["uu_date.exe"]="date.exe"
    ["uu_dd.exe"]="dd.exe"
    ["uu_df.exe"]="df.exe"
    ["uu_dir.exe"]="dir.exe"
    ["uu_dircolors.exe"]="dircolors.exe"
    ["uu_dirname.exe"]="dirname.exe"
    ["uu_du.exe"]="du.exe"
    ["uu_echo.exe"]="echo.exe"
    ["uu_env.exe"]="env.exe"
    ["uu_expand.exe"]="expand.exe"
    ["uu_expr.exe"]="expr.exe"
    ["uu_factor.exe"]="factor.exe"
    ["uu_false.exe"]="false.exe"
    ["uu_fmt.exe"]="fmt.exe"
    ["uu_fold.exe"]="fold.exe"
    ["uu_hashsum.exe"]="hashsum.exe"
    ["uu_head.exe"]="head.exe"
    ["uu_hostname.exe"]="hostname.exe"
    ["uu_join.exe"]="join.exe"
    ["uu_link.exe"]="link.exe"
    ["uu_ln.exe"]="ln.exe"
    ["uu_ls.exe"]="ls.exe"
    ["uu_mkdir.exe"]="mkdir.exe"
    ["uu_mktemp.exe"]="mktemp.exe"
    ["uu_more.exe"]="more.exe"
    ["uu_mv.exe"]="mv.exe"
    ["uu_nl.exe"]="nl.exe"
    ["uu_nproc.exe"]="nproc.exe"
    ["uu_numfmt.exe"]="numfmt.exe"
    ["uu_od.exe"]="od.exe"
    ["uu_paste.exe"]="paste.exe"
    ["uu_pr.exe"]="pr.exe"
    ["uu_printenv.exe"]="printenv.exe"
    ["uu_printf.exe"]="printf.exe"
    ["uu_ptx.exe"]="ptx.exe"
    ["uu_pwd.exe"]="pwd.exe"
    ["uu_readlink.exe"]="readlink.exe"
    ["uu_realpath.exe"]="realpath.exe"
    ["uu_rm.exe"]="rm.exe"
    ["uu_rmdir.exe"]="rmdir.exe"
    ["uu_seq.exe"]="seq.exe"
    ["uu_shred.exe"]="shred.exe"
    ["uu_shuf.exe"]="shuf.exe"
    ["uu_sleep.exe"]="sleep.exe"
    ["uu_sort.exe"]="sort.exe"
    ["uu_split.exe"]="split.exe"
    ["uu_sum.exe"]="sum.exe"
    ["uu_sync.exe"]="sync.exe"
    ["uu_tac.exe"]="tac.exe"
    ["uu_tail.exe"]="tail.exe"
    ["uu_tee.exe"]="tee.exe"
    ["uu_test.exe"]="test.exe"
    ["uu_touch.exe"]="touch.exe"
    ["uu_tr.exe"]="tr.exe"
    ["uu_true.exe"]="true.exe"
    ["uu_truncate.exe"]="truncate.exe"
    ["uu_tsort.exe"]="tsort.exe"
    ["uu_unexpand.exe"]="unexpand.exe"
    ["uu_uniq.exe"]="uniq.exe"
    ["uu_unlink.exe"]="unlink.exe"
    ["uu_vdir.exe"]="vdir.exe"
    ["uu_wc.exe"]="wc.exe"
    ["uu_whoami.exe"]="whoami.exe"
    ["uu_yes.exe"]="yes.exe"

    # Derive utilities (3 utilities)
    ["where.exe"]="where.exe"
    ["which.exe"]="which.exe"
    ["tree.exe"]="tree.exe"
)

# Function to backup and copy a utility
deploy_utility() {
    local src_name="$1"
    local dst_name="$2"
    local src_path="${WINUTILS_DIR}/target/release/${src_name}"
    local dst_path="${TARGET_DIR}/${dst_name}"

    # Check if source exists
    if [[ ! -f "${src_path}" ]]; then
        echo "  ⚠️  Source not found: ${src_name}"
        return 1
    fi

    # Backup existing file if it exists
    if [[ -f "${dst_path}" ]]; then
        local backup_name="${dst_name%.exe}_${TIMESTAMP}.exe"
        cp "${dst_path}" "${ARCHIVE_DIR}/${backup_name}"
        echo "  📦 Backed up existing ${dst_name} to .archive/${backup_name}"
    fi

    # Copy new file
    cp "${src_path}" "${dst_path}"
    chmod +x "${dst_path}"

    # Get file size for display
    local size=$(stat -c%s "${dst_path}" 2>/dev/null || echo "0")
    local size_kb=$((size / 1024))

    echo "  ✅ Deployed ${src_name} → ${dst_name} (${size_kb} KB)"
    return 0
}

# Count statistics
total=0
successful=0
backed_up=0
failed=0

echo "Starting deployment..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo

# Deploy all utilities
for src_name in "${!UTILITIES[@]}"; do
    dst_name="${UTILITIES[$src_name]}"
    total=$((total + 1))

    # Check if backup is needed
    if [[ -f "${TARGET_DIR}/${dst_name}" ]]; then
        backed_up=$((backed_up + 1))
    fi

    if deploy_utility "${src_name}" "${dst_name}"; then
        successful=$((successful + 1))
    else
        failed=$((failed + 1))
    fi
done

echo
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Deployment Summary:"
echo "  Total utilities:     ${total}"
echo "  Successfully deployed: ${successful}"
echo "  Files backed up:     ${backed_up}"
echo "  Failed:              ${failed}"
echo
echo "Installation directory: ${TARGET_DIR}"
echo "Backup directory:       ${ARCHIVE_DIR}"
echo

# Verify installation
echo "Verifying installation..."
echo "Sample commands:"
echo "  ${TARGET_DIR}/cat.exe --version"
echo "  ${TARGET_DIR}/ls.exe --version"
echo "  ${TARGET_DIR}/where.exe --version"
echo

# Test a sample command
if [[ -f "${TARGET_DIR}/cat.exe" ]]; then
    echo "Testing cat.exe:"
    "${TARGET_DIR}/cat.exe" --version | head -1
fi

echo
echo "✨ Deployment complete!"
