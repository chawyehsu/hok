# Changelog

## [0.1.0-alpha.2](https://github.com/chawyehsu/hok/compare/v0.1.0-alpha.1...v0.1.0-alpha.2) (2023-07-25)


### ⚠ BREAKING CHANGES

* **libscoop:** APIs of operations and Session changed.
* **libscoop:** exposed modules of libscoop changed.

### Features

* **hok|cat,home:** support candidate selection ([28b56c5](https://github.com/chawyehsu/hok/commit/28b56c5ade13e1edceb04fa7c0fc7554dcc0c6a9))
* **hok:** add uninstall cmd placeholder ([c13e8be](https://github.com/chawyehsu/hok/commit/c13e8be627ab0bfb91aedfebc10ee89dc2ee8675))
* **hok:** support list held packages ([a2acb22](https://github.com/chawyehsu/hok/commit/a2acb2210bf0586f6d839d61773b1dac7d2f96f1))
* **libscoop|manifest:** support aarch64 specific fields ([639d092](https://github.com/chawyehsu/hok/commit/639d092e22dc32decc98950532614da75489dbe6))
* **libscoop|resolve:** added fn `select_candidate` ([0e296ea](https://github.com/chawyehsu/hok/commit/0e296ea5b0cb2ab884c74ccea42df86ca05840e0))
* **libscoop:** add package resolving and event bus ([434eebe](https://github.com/chawyehsu/hok/commit/434eebe3d464edb48a1d034d4e746810ba41d274))
* **libscoop:** replace ureq with libcurl ([7d3df7c](https://github.com/chawyehsu/hok/commit/7d3df7c3e954187318d46958f07d6e4b4ce9fe31))
* move to v0.1.0-alpha.2 ([24e354a](https://github.com/chawyehsu/hok/commit/24e354a7514d74878c550e25457d323e6251ee4b))


### Bug Fixes

* **hok|cat,home:** sort candidates ([c90f3f9](https://github.com/chawyehsu/hok/commit/c90f3f94367dae75cabd2dd0a562f38c924f6dbd))
* **libscoop:** dag should check self cyclic ([a5bbb0b](https://github.com/chawyehsu/hok/commit/a5bbb0bb5f57ef6d8d326e6eca9bb828f6ff6ec9))


### Miscellaneous Chores

* **libscoop:** tweak exposed modules ([f31cb64](https://github.com/chawyehsu/hok/commit/f31cb64d3794edf01b55757bb3ecdc19d4878932))

## 0.1.0-alpha.1 (2023-07-21)


### Features

* add hash crate ([aa021fb](https://github.com/chawyehsu/hok/commit/aa021fb7fa6eaa3167f803608982307ebbafe9f7))
* **api:** Introduce SPDX spec for manifest.license ([ec5e1f5](https://github.com/chawyehsu/hok/commit/ec5e1f5c6286100724f346ab55ab7fc11d02d5fe))
* **cache:** implement cache-rm ([869f095](https://github.com/chawyehsu/hok/commit/869f0956a0ccb6a8dc06d40d95bde9f79b09e504))
* **cmd:** Implement cleanup, refactor cache and list ([3ca3f26](https://github.com/chawyehsu/hok/commit/3ca3f2610ec5bf0164bfde1d4f91484423cc78c4))
* **cmd:** Implement scoop list subcommand ([0b2fdec](https://github.com/chawyehsu/hok/commit/0b2fdec835835b68b500a19d450f39e82c08a4b6))
* **cmd:** prototype of scoop home ([29c2663](https://github.com/chawyehsu/hok/commit/29c2663768e7bed616e104c6a5339b55bcdf7536))
* **cmd:** prototype of scoop info ([a207465](https://github.com/chawyehsu/hok/commit/a207465b73a704ef31014ccd408c323c45cbbdb5))
* **cmd:** prototype of scoop search (local) ([2c8e563](https://github.com/chawyehsu/hok/commit/2c8e563748539b63e6c95b9c09dbe9b1b1995199))
* **core:** add DepGraph implementation ([ca5f49f](https://github.com/chawyehsu/hok/commit/ca5f49fcd23437a5d257fd83fd23cf1c512cdb27))
* **core:** Implement update subcommand ([ad04e76](https://github.com/chawyehsu/hok/commit/ad04e76762de55954d070be3a3a352b29a78981e))
* **hash-md5:** add reset api ([3db7116](https://github.com/chawyehsu/hok/commit/3db7116412729ff2ca84de93ecd1a1850e17100e))
* **hash:** add checksum helper functions ([df24980](https://github.com/chawyehsu/hok/commit/df24980c664699b24a2efc7b609c2ba324521333))
* **hash:** add sha1 implementation ([8bee89a](https://github.com/chawyehsu/hok/commit/8bee89ae49f30cfbdb42c76c52331c7fd5ba8b82))
* **hash:** add sha256 implementation ([37f9f62](https://github.com/chawyehsu/hok/commit/37f9f622e79a5ec4d3bf122ccafd191d46041c2b))
* **hash:** add sha512 implementation ([7bbecf1](https://github.com/chawyehsu/hok/commit/7bbecf1310ee342e4d4376e413f852b16f6aadd2))
* **hash:** provided a top-level checksum api ([99fed09](https://github.com/chawyehsu/hok/commit/99fed093d48d5cf91f3db0f46f02c4d152d17043))
* Implement basic file downloads ([c5d303b](https://github.com/chawyehsu/hok/commit/c5d303bff23993ca4bc53946c074058a542a0420))
* implement hold and unhold ([682c63c](https://github.com/chawyehsu/hok/commit/682c63c78390ee4300a6c9ad42934b79be7b5866))
* implement status ([bb650d6](https://github.com/chawyehsu/hok/commit/bb650d64c711f74ff1f73c3026b86c90daafe14b))
* **scoop-cache:** implement scoop cache show ([ae018b8](https://github.com/chawyehsu/hok/commit/ae018b86a3abfe23d4f6f9c17edc9047947af8e4))
* **scoop-cache:** implement scoop cache show ([c584c90](https://github.com/chawyehsu/hok/commit/c584c90ff3e90e8744841ea64e3f732a29571b55))
* **scoop-config:** implement scoop config ([9bdc9fa](https://github.com/chawyehsu/hok/commit/9bdc9fa8a46897dea3aef636bd92d51a27b7616f))
* **search:** Add fuzzy search option ([53c8998](https://github.com/chawyehsu/hok/commit/53c8998ed98b4a150e19ffb4a10ce7a7e8ab160e))
* **search:** Implement binary search ([26ff6f2](https://github.com/chawyehsu/hok/commit/26ff6f248fc323f5be1e168e10d19fff613e07a9))
* update ([cf505f0](https://github.com/chawyehsu/hok/commit/cf505f0e51ac4c5777e260651d0ee0cd5e805abb))
* v0.1.0-alpha.1 ([f304bb2](https://github.com/chawyehsu/hok/commit/f304bb262dc1f850ae3932bb810ab91ee272fd2b))


### Bug Fixes

* **bucket-rm:** use remove_dir_all crate ([a5c9a0b](https://github.com/chawyehsu/hok/commit/a5c9a0bb309a54bae2e80552d4a5c9c0b5a4ef16))
* **core:** fix cache regex ([98f2a44](https://github.com/chawyehsu/hok/commit/98f2a44d872c876c6925e6a5ffadbc4864ddfb71))
* **core:** fix manifest download urls extraction ([7fef94c](https://github.com/chawyehsu/hok/commit/7fef94cb1235ce446d10bf0ab09bc853fc1ccd0e))
* Fix apps_in_local_bucket ([b3170c7](https://github.com/chawyehsu/hok/commit/b3170c72263dabacb027e8b66b1be3ce7113bfb7))
* fix cache rm handler ([22f51e2](https://github.com/chawyehsu/hok/commit/22f51e2cb82a80d1123895452ed2cecbf0d09b4a))
* Fix not truncating previous data when saving new configs ([ea4bf0c](https://github.com/chawyehsu/hok/commit/ea4bf0c1fa28d7ede20d35cedc5423c515a4a029))
* typo ([4ddc72f](https://github.com/chawyehsu/hok/commit/4ddc72f944d1fa235fd9644e9ec7896cf917ccc3))
* use init method to create config instance ([9aa08ba](https://github.com/chawyehsu/hok/commit/9aa08ba9caedefb62b86c4c70593463aacaeefae))


### Performance Improvements

* don't update install info if it's not held ([d8b71b1](https://github.com/chawyehsu/hok/commit/d8b71b117d97380085b14c145a59495e0ccae5f3))
* **hash-md5:** use inline fn for performance ([7f35660](https://github.com/chawyehsu/hok/commit/7f356602a93091da5b0017de3a0cb00b3a0e1bb4))
* search enhancement ([13075cc](https://github.com/chawyehsu/hok/commit/13075cc11a267d98296541fdaf3582b2f9f50eca))
