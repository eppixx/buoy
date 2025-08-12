#!/bin/bash

ARG="$1"

# colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

# input:
# 1 - color of output
# 2 - string to print
function print_color()
{
  echo -e "$1$2 $NC"
}

function build_all() {
    deb_build
    rpm_build
}

function clean_all() {
    deb_clean
    rpm_clean
}

function deb_build() {
    # translate desktop file
    meson setup build-deb && ninja -C build-deb data/com.github.eppixx.buoy.desktop

    # build and package deb file
    cargo deb

    # clean up meson folder
    rm -rf build-deb

    print_color $GREEN "deb file is in folder target/debian"
}

function deb_clean() {
    print_color $GREEN "clean deb"
    rm -rf build-deb
    rm -rf target/debian
}

function rpm_build() {
    # translate desktop file
    meson setup build-rpm && ninja -C build-rpm data/com.github.eppixx.buoy.desktop

    # build
    cargo build --release
    # package rpm file
    cargo generate-rpm

    rm -rf build-rpm

    print_color $GREEN "rpm file is in folder target/generate-rpm"
}

function rpm_clean() {
    print_color $GREEN "clean rpm"
    rm -rf build-rpm
    rm -rf target/generate-rpm
}

# show options
function choose() {
    if [ "$ARG" == "" ] ; then
        print_color $GREEN "What do you want to do?"
        echo "1 - build deb"
        echo "2 - clean deb"
        echo "3 - build rpm"
        echo "4 - clean rpm"
        echo "9 - build all"
        echo "0 - clean all"
        echo "x - exit"
        read ARG
    fi

    case "$ARG" in
        1) deb_build;;
        2) deb_clean;;
        3) rpm_build;;
        4) rpm_clean;;
        9) build_all;;
        0) clean_all;;
        x | X) exit;;
        *) choose;;
    esac
}

choose
