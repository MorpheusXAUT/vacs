# Changelog

## [0.3.1](https://github.com/MorpheusXAUT/vacs/compare/vacs-webrtc-v0.3.0...vacs-webrtc-v0.3.1) (2025-12-15)

## [0.3.0](https://github.com/MorpheusXAUT/vacs/compare/vacs-webrtc-v0.2.0...vacs-webrtc-v0.3.0) (2025-11-30)


### Features

* provide TURN servers for traversing restrictive networks ([#248](https://github.com/MorpheusXAUT/vacs/issues/248)) ([e4b8b91](https://github.com/MorpheusXAUT/vacs/commit/e4b8b91320fd6d072ef4ba1c98de56ad14c8dcfe))
* **vacs-client:** add profile select to mission page ([ad36dc5](https://github.com/MorpheusXAUT/vacs/commit/ad36dc55e2e42619eff9c0163e869f64910998bb))
* **vacs-client:** add station filter and aliasing ([#233](https://github.com/MorpheusXAUT/vacs/issues/233)) ([ad36dc5](https://github.com/MorpheusXAUT/vacs/commit/ad36dc55e2e42619eff9c0163e869f64910998bb))
* **vacs-client:** load ICE config after signaling connect ([e4b8b91](https://github.com/MorpheusXAUT/vacs/commit/e4b8b91320fd6d072ef4ba1c98de56ad14c8dcfe))
* **vacs-webrtc:** use shared IceConfig types ([e4b8b91](https://github.com/MorpheusXAUT/vacs/commit/e4b8b91320fd6d072ef4ba1c98de56ad14c8dcfe))

## [0.2.0](https://github.com/MorpheusXAUT/vacs/compare/vacs-webrtc-v0.1.1...vacs-webrtc-v0.2.0) (2025-11-09)


### Features

* **vacs-audio:** implement DeviceSelector with improved device support ([5d3999a](https://github.com/MorpheusXAUT/vacs/commit/5d3999ae6ab833cfb52d82bb914632feb686ade9))
* **vacs-client:** WIP webrtc manager impl ([9be6c17](https://github.com/MorpheusXAUT/vacs/commit/9be6c17d893e047037b6a3634700041e99c4e941))
* **vacs-webrtc:** abstract webrtc dependency and implement ICE candidate trickling ([c722967](https://github.com/MorpheusXAUT/vacs/commit/c7229670edd111157adf0d1ef84ed30eff8ba3e5))


### Bug Fixes

* **vacs-webrtc:** implement pausing and resuming for webrtc peer ([33f7c14](https://github.com/MorpheusXAUT/vacs/commit/33f7c14add0e410fe82a9c43b32da1e5c209aa5d)), closes [#8](https://github.com/MorpheusXAUT/vacs/issues/8)
