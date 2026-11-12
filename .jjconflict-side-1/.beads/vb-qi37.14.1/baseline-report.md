# Baseline Report - vb-qi37.14.1

## Bead
- **Bead ID**: vb-qi37.14.1
- **Title**: cli: Add single-step run command
- **Baseline Date**: 2026-05-18

## Baseline Commit
- **Main Branch**: 0a4d1e4980705d79b17bcd1b09a8fdab8cc8e030
- **Description**: fix(vb_runtime): tighten admission test assertions

## Bead Description
Implement run --step for deterministic single-step workflow execution against accepted artifacts with structured output and typed errors.

## Acceptance Criteria
- run --step executes exactly one step
- Reports pc/slot/taint/state deltas
- Respects durability gates
- Has tests for valid and invalid step requests

## Existing CLI Structure
The bead is part of the CLI tool for velvet-ballistics. Need to explore:
- How existing CLI commands are structured
- How workflow execution currently works
- How step execution could be added as a single-step mode

## Dependencies
- vb-qi37.13.4: cli: Structured output contract tests (COMPLETE)

## Blocks
- vb-qi37.14: cli: Prove explain, diff, graph, and run-step contracts
