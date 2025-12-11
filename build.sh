#!/bin/bash

set -e

cargo build --release
cd riscv-vm && yarn build && cd ..
