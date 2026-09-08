# Task

Add a flag to override the auto-derived step across every orksorksorks command that currently determines it by inspecting trigger artifacts. Today `step` (on main) and—on the unmerged `add-model-command` branch—`model`, `thinking`, and `prompt` all call the shared `determine_step`; the flag must let each of these commands be told which step to use instead of inferring it from the artifact directory. The root puzzle is where the override lives (one global flag vs. per-subcommand), how an unknown step name is validated against `config.steps`, and which base this change actually lands on.

## Why LARGE

Matched triggers: **MULTI_MODULE**, **UNKNOWNS**, **DESIGN_SIGN-OFF**, **CONVENTION_RISK**. The change spans the whole step-deriving command family (step + prompt + model + thinking handlers, their shared `determine_step`/CLI-dispatch code, and 2–4 integration test files), the `prompt` surface isn't even merged onto main (dependency/ordering unknown), and flag placement (global `--step` vs. per-command) plus validation semantics are open design decisions touching shared convention code.