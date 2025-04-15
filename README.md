# secure-cord

A ratatui port of secure-wires. A SSM client, in the vein of [secure pipes](https://www.opoet.com/pyro/index.php)

## TODO:

 - [ ] Remove the need for a refresh loop to poll the state of sessions; we should be able to instead call back with the state
 - [ ] Implement an actual health check, right now we just rely on session-manager dying

## Lessons:

 - Using Actors seems to have encapsulated the state of the process pretty well, but it was moot as using async to get the state is not tenable. Seems inevitable that we have to have the state inside the UI layer, and use channels to subscribe to updates, like we did in Kerf. It tightly couples the UI to the logic, however, so not a fan...
 - Confining state updates inside the parent UI struct, did solve a few borrow issues. I'm not sold on using indexes instead of references, but it's a common ratatui pattern here, and I may just be stubborn.
