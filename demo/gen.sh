#!/usr/bin/env bash

RENDER_OPTS="--stroke red --stroke-width 3.0"

svgps generate ./ferris.svg ./ferris.svgcom
svgps render ./ferris.svgcom ./ferris-converted.svg $RENDER_OPTS

svgps generate ./ferris.svg ./ferris.svgcom --autocut
svgps render ./ferris.svgcom ./ferris-converted-autocut.svg $RENDER_OPTS

svgps generate ./tiger.svg ./tiger.svgcom
svgps render ./tiger.svgcom ./tiger-converted.svg $RENDER_OPTS

svgps generate ./tiger.svg ./tiger.svgcom --autocut
svgps render ./tiger.svgcom ./tiger-converted-autocut.svg $RENDER_OPTS