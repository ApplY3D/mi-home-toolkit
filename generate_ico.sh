#!/bin/bash
IFS='.' read -ra ADDR <<< "$1"
ICONSET=${ADDR[0]}.iconset.ico
mkdir $ICONSET
sips -z 16 16     $1 --out $ICONSET/16x16.png
sips -z 24 24     $1 --out $ICONSET/24x24.png
sips -z 32 32     $1 --out $ICONSET/32x32.png
sips -z 48 48     $1 --out $ICONSET/48x48.png
sips -z 64 64     $1 --out $ICONSET/64x64.png
sips -z 256 256   $1 --out $ICONSET/256x256.png
convert $ICONSET/16x16.png $ICONSET/24x24.png $ICONSET/32x32.png $ICONSET/48x48.png $ICONSET/64x64.png $ICONSET/256x256.png -compress jpeg icon.ico 
rm -R $ICONSET