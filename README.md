# Bloom

Command utility to filter out duplicated lines using bloom filter method.

## Overview

Bloom is a command-line tool aims at printing only unique lines from the
standard input. It uses [Bloom filter][bf-wiki], a space-efficient
probabilistic data structure used to test whether a given element is a member
of a set.  The tool provides functionality to read lines from standard input,
calculate check sums, and insert them into a Bloom filter. Additionally, it
supports the option to load and update existing Bloom filter files, set
limits on the number of lines inserted, and more.

## Build

To build from source, clone this repository and run:

    cargo build

To build using optimized profile, run:

    cargo build --profile optimized

## Install

To install, run:

    cargo install --git https://github.com/kenorb/bloom.git

## Examples

    # Filters out duplicated lines.
    $ (seq 10; seq 10) | bloom | wc -l
    10
    # Prints lines only when present in bloom filter.
    $ (seq 10; seq 10) | bloom -v | wc -l
    10
    # Store maximum 9 lines.
    $ seq 10 | bloom -l 9 | wc -l
    10
    # Writes bloom filter into the file, then use it again to filter out lines.
    $ seq 10 | bloom -f 10.blf -w; seq 10 | bloom -f 10.blf | wc -l
    0

<!-- Named links -->

[bf-wiki]: https://en.wikipedia.org/wiki/Bloom_filter
