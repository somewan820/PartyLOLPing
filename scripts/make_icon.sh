#!/bin/zsh
set -euo pipefail

destination="${1:?usage: make_icon.sh ICONSET_DIRECTORY}"
source_image="${0:A:h:h}/Resources/Icons/ping.png"

rm -rf "$destination"
mkdir -p "$destination"

for size in 16 32 128 256 512; do
    for factor in 1 2; do
        pixels=$((size * factor))
        filename="icon_${size}x${size}"
        if (( factor == 2 )); then
            filename+="@2x"
        fi
        sips -z "$pixels" "$pixels" "$source_image" \
            --out "$destination/${filename}.png" >/dev/null
    done
done
