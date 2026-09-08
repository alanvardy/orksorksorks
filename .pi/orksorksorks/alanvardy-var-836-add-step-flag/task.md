# Task

Add a flag that lets the user override the auto-derived step for every orksorksorks command that currently determines its step by inspecting trigger artifacts in the artifact directory (on the current branch: `step`; on the unmerged `add-model-command` branch: `model`, `thinking`, and `prompt`). Instead of inferring the step from which artifact files exist, the user can tell a command which step to use directly, and unknown step names should be handled consistently with the existing validation/error paths.
