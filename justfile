set shell := ["bash", "-euo", "pipefail", "-c"]

default:
    @just --list

ci:
    moon ci

fmt:
    moon run :fmt

lint:
    moon run :lint-src

check:
    moon run :check

test:
    moon run :test

quick:
    moon run :quick

nightly-feature-gate:
    moon run :nightly-feature-gate

nightly-feature-cargo-probe:
    moon run :nightly-feature-cargo-probe
