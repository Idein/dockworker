# Dockworker: Rust library for talking to the Docker daemon

[![CircleCI](https://circleci.com/gh/eldesh/dockworker.svg?style=svg)](https://circleci.com/gh/eldesh/dockworker)
[![Build status](https://ci.appveyor.com/api/projects/status/88ut6hplkw7vtjy4?svg=true)](https://ci.appveyor.com/project/eldesh/dockworker)

## Support

### Environment

- Docker
    - API 1.26

- OS
    - Linux (developped in Ubuntu(amd64))
    - Windows

### Api

Supported Api List.
`Support` means that any wrapper method exists in this crate.

- container
	- [x] `/containers/json`
	- [x] `/containers/create`
	- [x] `/containers/{id}/json`
	- [x] `/containers/{id}/top`
	- [ ] `/containers/{id}/logs`
	- [x] `/containers/{id}/changes`
	- [x] `/containers/{id}/export`
	- [x] `/containers/{id}/stats`
	- [ ] `/containers/{id}/resize`
	- [x] `/containers/{id}/start`
	- [x] `/containers/{id}/stop`
	- [ ] `/containers/{id}/restart`
	- [x] `/containers/{id}/kill`
	- [ ] `/containers/{id}/update`
	- [ ] `/containers/{id}/rename`
	- [ ] `/containers/{id}/pause`
	- [ ] `/containers/{id}/unpause`
	- [x] `/containers/{id}/attach`
	- [ ] `/containers/{id}/attach/ws`
	- [x] `/containers/{id}/wait`
	- [x] `/containers/{id}` # remove
	- [ ] `/containers/{id}/archive`
	- [ ] `/containers/{id}/prune`

- image
	- [x] `/images/json`
	- [ ] `/build`
	- [ ] `/build/prune`
	- [x] `/images/create`
	- [ ] `/images/{name}/json`
	- [ ] `/images/{name}/history`
	- [x] `/images/{name}/push`
	- [ ] `/images/{name}/tag`
	- [x] `/images/{name}` # remove
	- [ ] `/images/search`
	- [ ] `/images/prune`
	- [ ] `/commit`
	- [x] `/images/{name}/get`
	- [ ] `/images/get`
	- [x] `/images/load`

- system
	- [x] `/auth`
	- [x] `/info`
	- [x] `/version`
	- [x] `/_ping`
	- [ ] `/events`
	- [ ] `/system/df`


## Original Project Contributors

`Dockworker` crate is forked from [boondock](https://github.com/faradayio/boondock).
Heres are contributors to it.

- Graham Lee <ghmlee@ghmlee.com>
- Toby Lawrence <toby@nuclearfurnace.com>
- Eric Kidd <git@randomhacks.net>

