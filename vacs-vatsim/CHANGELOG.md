# Changelog

## [1.0.0](https://github.com/MorpheusXAUT/vacs/compare/vacs-vatsim-v0.2.1...vacs-vatsim-v1.0.0) (2026-02-20)


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

## [0.2.1](https://github.com/MorpheusXAUT/vacs/compare/vacs-vatsim-v0.2.0...vacs-vatsim-v0.2.1) (2025-12-05)


### Bug Fixes

* **vacs-vatsim:** fix unknown station handling with visibility range set ([#284](https://github.com/MorpheusXAUT/vacs/issues/284)) ([ab44a8a](https://github.com/MorpheusXAUT/vacs/commit/ab44a8ad64a393afb431274e23819ca23cefbb90))

## [0.2.0](https://github.com/MorpheusXAUT/vacs/compare/vacs-vatsim-v0.1.0...vacs-vatsim-v0.2.0) (2025-11-09)


### Features

* **vacs-server:** add mock data feed ([c3ba168](https://github.com/MorpheusXAUT/vacs/commit/c3ba168ddfc5350d4c7b847d1bcff067a27f7416))
* **vacs-vatsim:** implement facility type parsing ([4ff59b3](https://github.com/MorpheusXAUT/vacs/commit/4ff59b31b59621b2a735880b533ca851f66ffc65))
* **vacs-vatsim:** implement frequency parsing from sluper API ([9a1c95f](https://github.com/MorpheusXAUT/vacs/commit/9a1c95f815f40dec5d83573d86fd9bf3e29b06f7))
* **vacs-vatsim:** implement VATSIM data feed parsing ([146df1c](https://github.com/MorpheusXAUT/vacs/commit/146df1c095da120e3cf073fd864a41dec744db85))


### Bug Fixes

* fix tests after login refactor ([8d2c2d6](https://github.com/MorpheusXAUT/vacs/commit/8d2c2d626c75acf15dd6dc771315b3816cf209fe))
