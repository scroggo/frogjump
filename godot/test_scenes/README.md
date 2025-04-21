# Test bed

## Operation

Run `test_main.tscn` to run through all of the test scenes. `ESCAPE`
reloads the current scene and `ENTER` transitions to the next scene.
When making behavior changes, cycle through the test scenes to avoid
regressions.

## New test scenes

When a bug is found, add a new test scene that exemplifies the bug in
this folder. Add it to the "Scenes" array in `test_main.tscn` to be
included by future regression testing.

Note: The first scene, `test_bed` is an introduction, and the final
three are related, with the very final scene demonstrating the message
shown when beating the final level. So new scenes should be added after
the first scene and before the final three.
