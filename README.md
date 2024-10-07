# Dockworker: Rust library for talking to the Docker daemon

[![CircleCI](https://circleci.com/gh/Idein/dockworker/tree/master.svg?style=svg)](https://circleci.com/gh/Idein/dockworker/tree/master)
[![Build status](https://ci.appveyor.com/api/projects/status/88ut6hplkw7vtjy4/branch/master?svg=true)](https://ci.appveyor.com/project/eldesh/dockworker)

## Support

### Environment

- Docker
    - API version 1.26

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
	- [x] `/containers/{id}/logs`
	- [x] `/containers/{id}/changes`
	- [x] `/containers/{id}/export`
	- [x] `/containers/{id}/exec`
	- [x] `/containers/{id}/stats`
	- [ ] `/containers/{id}/resize`
	- [x] `/containers/{id}/start`
	- [x] `/containers/{id}/stop`
	- [x] `/containers/{id}/restart`
	- [x] `/containers/{id}/kill`
	- [ ] `/containers/{id}/update`
	- [ ] `/containers/{id}/rename`
	- [ ] `/containers/{id}/pause`
	- [ ] `/containers/{id}/unpause`
	- [x] `/containers/{id}/attach`
	- [ ] `/containers/{id}/attach/ws`
	- [x] `/containers/{id}/wait`
	- [x] `/containers/{id}` # remove
	- [x] `/containers/{id}/archive`
	- [ ] `/containers/{id}/prune`

- checkpoints
    - [x] `/containers/{id}/checkpoints`

- exec
    - [x] `/exec/{id}/start`
    - [x] `/exec/{id}/json`

- image
	- [x] `/images/json`
	- [x] `/build`
	- [ ] `/build/prune`
	- [x] `/images/create`
	- [x] `/images/{name}/json`
	- [x] `/images/{name}/history`
	- [x] `/images/{name}/push`
	- [ ] `/images/{name}/tag`
	- [x] `/images/{name}` # remove
	- [x] `/images/search`
	- [x] `/images/prune`
	- [ ] `/commit`
	- [x] `/images/{name}/get`
	- [ ] `/images/get`
	- [x] `/images/load`

- system
	- [x] `/auth`
	- [x] `/info`
	- [x] `/version`
	- [x] `/_ping`
	- [x] `/events`
	- [ ] `/system/df`

- network
	- [x] `/networks`
	- [x] `/networks/{id}`
	- [x] `/networks/{id}` # remove
	- [x] `/networks/create`
	- [x] `/networks/{id}/connect`
	- [x] `/networks/{id}/disconnect`
	- [x] `/networks/prune`


## Test

Executing unit tests:

```shell
$ cargo test
```

### Depends on docker

Some test cases depend on docker are disabled by default.
These containers required from test cases are built by `docker-compose` like below:

```shell
$ docker-compose build
$ cargo test -- --ignored
```


## Original Project Contributors

`Dockworker` crate is forked from [boondock](https://github.com/faradayio/boondock).
Heres are contributors to it.

- Graham Lee <ghmlee@ghmlee.com>
- Toby Lawrence <toby@nuclearfurnace.com>
- Eric Kidd <git@randomhacks.net>

