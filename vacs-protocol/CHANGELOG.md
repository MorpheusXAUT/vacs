# Changelog

## [2.0.0](https://github.com/MorpheusXAUT/vacs/compare/vacs-protocol-v1.1.0...vacs-protocol-v2.0.0) (2026-02-20)


### ⚠ BREAKING CHANGES

* implement station coverage calculations and calling ([#452](https://github.com/MorpheusXAUT/vacs/issues/452))
* overhaul UI with geo/tabbed layout and station-based calling ([#531](https://github.com/MorpheusXAUT/vacs/issues/531))

### Features

* add priority calls ([#504](https://github.com/MorpheusXAUT/vacs/issues/504)) ([384131b](https://github.com/MorpheusXAUT/vacs/commit/384131bf18dbe8240602d6f4e0b226fb04effdf3))
* implement station coverage calculations and calling ([#452](https://github.com/MorpheusXAUT/vacs/issues/452)) ([384131b](https://github.com/MorpheusXAUT/vacs/commit/384131bf18dbe8240602d6f4e0b226fb04effdf3))
* overhaul UI with geo/tabbed layout and station-based calling ([#531](https://github.com/MorpheusXAUT/vacs/issues/531)) ([384131b](https://github.com/MorpheusXAUT/vacs/commit/384131bf18dbe8240602d6f4e0b226fb04effdf3))
* **vacs-client:** add call start and end sounds ([#505](https://github.com/MorpheusXAUT/vacs/issues/505)) ([384131b](https://github.com/MorpheusXAUT/vacs/commit/384131bf18dbe8240602d6f4e0b226fb04effdf3))
* **vacs-client:** add keybind for toggling radio prio ([#500](https://github.com/MorpheusXAUT/vacs/issues/500)) ([384131b](https://github.com/MorpheusXAUT/vacs/commit/384131bf18dbe8240602d6f4e0b226fb04effdf3))
* **vacs-client:** add window zoom hotkeys ([#522](https://github.com/MorpheusXAUT/vacs/issues/522)) ([384131b](https://github.com/MorpheusXAUT/vacs/commit/384131bf18dbe8240602d6f4e0b226fb04effdf3))
* **vacs-client:** implement telephone directory ([#490](https://github.com/MorpheusXAUT/vacs/issues/490)) ([384131b](https://github.com/MorpheusXAUT/vacs/commit/384131bf18dbe8240602d6f4e0b226fb04effdf3))


### Bug Fixes

* **vacs-client:** fix error while switching to exclusive audio device ([#498](https://github.com/MorpheusXAUT/vacs/issues/498)) ([384131b](https://github.com/MorpheusXAUT/vacs/commit/384131bf18dbe8240602d6f4e0b226fb04effdf3))
* **vacs-client:** prevent call queue from shrinking ([384131b](https://github.com/MorpheusXAUT/vacs/commit/384131bf18dbe8240602d6f4e0b226fb04effdf3))
* **vacs-server:** ignore datafeed SUP connection ([#480](https://github.com/MorpheusXAUT/vacs/issues/480)) ([384131b](https://github.com/MorpheusXAUT/vacs/commit/384131bf18dbe8240602d6f4e0b226fb04effdf3))

## [1.1.0](https://github.com/MorpheusXAUT/vacs/compare/vacs-protocol-v1.0.0...vacs-protocol-v1.1.0) (2025-11-30)


### Features

* provide TURN servers for traversing restrictive networks ([#248](https://github.com/MorpheusXAUT/vacs/issues/248)) ([e4b8b91](https://github.com/MorpheusXAUT/vacs/commit/e4b8b91320fd6d072ef4ba1c98de56ad14c8dcfe))
* **vacs-client:** load ICE config after signaling connect ([e4b8b91](https://github.com/MorpheusXAUT/vacs/commit/e4b8b91320fd6d072ef4ba1c98de56ad14c8dcfe))
* **vacs-server:** implement Prometheus metrics ([#251](https://github.com/MorpheusXAUT/vacs/issues/251)) ([b6d72fd](https://github.com/MorpheusXAUT/vacs/commit/b6d72fd6bfa719380efa966d55c02b85800978f6))
* **vacs-webrtc:** use shared IceConfig types ([e4b8b91](https://github.com/MorpheusXAUT/vacs/commit/e4b8b91320fd6d072ef4ba1c98de56ad14c8dcfe))


### Bug Fixes

* **vacs-server:** prevent clients from sending signaling messages to own peer_id ([#244](https://github.com/MorpheusXAUT/vacs/issues/244)) ([098ec4c](https://github.com/MorpheusXAUT/vacs/commit/098ec4cd0d79225b8542710199f79f3e9e84dac0))

## [1.0.0](https://github.com/MorpheusXAUT/vacs/compare/vacs-protocol-v0.1.0...vacs-protocol-v1.0.0) (2025-11-09)


### ⚠ BREAKING CHANGES

* **vacs-protocol:** add RateLimited error reason

### Features

* **vacs-client:** add auto-hangup for unanswered calls ([4f32f22](https://github.com/MorpheusXAUT/vacs/commit/4f32f22877371eaa10045f94d664aa1a81afcee3))
* **vacs-protocol:** add RateLimited error reason ([80cf829](https://github.com/MorpheusXAUT/vacs/commit/80cf829b206991962feb11b7ca9eea38dc92e728))
