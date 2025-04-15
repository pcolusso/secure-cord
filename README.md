# secure-cord

A ratatui port of secure-wires. A SSM client, in the vein of [secure pipes](https://www.opoet.com/pyro/index.php)

## TODO:

 - [ ] Remove the need for a refresh loop to poll the state of sessions; we should be able to instead call back with the state
 - [ ] Implement an actual health check, right now we just rely on session-manager dying
