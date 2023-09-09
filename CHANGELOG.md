# Changelog

## [0.1.0-beta.4](https://github.com/chawyehsu/hok/compare/v0.1.0-beta.3...v0.1.0-beta.4) (2023-09-09)


### Features

* **hok:** added s shortcut for search command ([50c0bfc](https://github.com/chawyehsu/hok/commit/50c0bfcd6dd928dc105a4ec7afefb1d4e0aa97c7))


### Bug Fixes

* **hok:** added long format arg of listing known buckets ([658ef7d](https://github.com/chawyehsu/hok/commit/658ef7d9e799301bbd5807a195dd2f263933d5c1))
* **hok:** fix 50c0bfc ([387fd66](https://github.com/chawyehsu/hok/commit/387fd66637d7e53d167a18be8a0fc9daf121475e))
* **hok:** trim yes_no prompt input ([0a01f1e](https://github.com/chawyehsu/hok/commit/0a01f1e1ee65e50f9cc5e081d8daa734ea7770e4))
* **libscoop|config:** default config path should be always returned ([d3040ad](https://github.com/chawyehsu/hok/commit/d3040adf732839bb0070f6585be5428ad0d25e73))
* **libscoop|fs:** improve symlink removal logic ([398ef27](https://github.com/chawyehsu/hok/commit/398ef27fc280ded401e0e5fb5a9123d5a165b2af))
* **libscoop|resolve:** correct pinned dependency cascade resolving ([660d3e2](https://github.com/chawyehsu/hok/commit/660d3e2da5bbe5218c45c8706282bfdbc2bfe760))


### Performance Improvements

* **libscoop|manifest:** defer hash validation ([d1ff3f6](https://github.com/chawyehsu/hok/commit/d1ff3f61a46b930771b0d4809fcf77ada2ac04c3))

## [0.1.0-beta.3](https://github.com/chawyehsu/hok/compare/v0.1.0-beta.2...v0.1.0-beta.3) (2023-08-09)


### ⚠ BREAKING CHANGES

* **libscoop|config:** `Package::manifest_path` is replaced by `manifest().path()`.

### Features

* **hok:** reflect basic support of uninstalling packages ([183cfd8](https://github.com/chawyehsu/hok/commit/183cfd8b54e8e96ce2e575240f3b7edb3183f005))
* **libscoop|sync:** basic support of uninstalling packages ([b1f0f6b](https://github.com/chawyehsu/hok/commit/b1f0f6bd3c7ee61b846d60a70889c4033730b10a))


### Bug Fixes

* **hok:** print ending newline for error report ([d1f5682](https://github.com/chawyehsu/hok/commit/d1f56822a1db93cf566265b3ec44082794896422))
* **libscoop|config:** correct `no_junction` field ([4bae700](https://github.com/chawyehsu/hok/commit/4bae700efa06b4d07506370e2eaace04ef747d3d))
* **libscoop|query:** don't create empty apps dir ([7287bd5](https://github.com/chawyehsu/hok/commit/7287bd5672f6eb88ebb52acb928bfbcc6e87877a))
* **libscoop:** added portability on non-windows ([3d1ffee](https://github.com/chawyehsu/hok/commit/3d1ffeeb39074a8c31cbb97891c082fd2a31a7fc))
* **libscoop:** avoid forcing doc target as it will fail to build ([7674f8a](https://github.com/chawyehsu/hok/commit/7674f8aef2f1cdf952c96aed6f16dbb08f65f335))
* **libscoop:** emit BucketUpdateDone event despite zero bucket ([dc4bdca](https://github.com/chawyehsu/hok/commit/dc4bdca39bec812073ca68f332d568e391736ef8))
* **libscoop:** ensure cache dir exist before downloading ([485255e](https://github.com/chawyehsu/hok/commit/485255e926df58efd6e03881d430c8496c9a4adb))
* **scoop-hash:** remove docsrs target ([b6ddd19](https://github.com/chawyehsu/hok/commit/b6ddd19f7c1f70754c70c3b1c6ca87c43e0e0754))

## [0.1.0-beta.2](https://github.com/chawyehsu/hok/compare/v0.1.0-beta.1...v0.1.0-beta.2) (2023-08-03)


### ⚠ BREAKING CHANGES

* **libscoop:** `SyncOption::NoDownloadSize` becomes `SyncOption::Offline`

### Features

* **hok|cat:** show manifest path ([7e06467](https://github.com/chawyehsu/hok/commit/7e064672ebd6aa2009f1db49ea6a0f8704139be3))
* **hok:** show bucket manifest count ([d71e193](https://github.com/chawyehsu/hok/commit/d71e193be2cc20598e53b08947635f67a1409399))
* **libscoop|download:** support injecting cookie defined in manifest ([aec7fdc](https://github.com/chawyehsu/hok/commit/aec7fdc851aee1673170182f7d382a069d514649))
* **libscoop|download:** write to temp file in downloading ([d79e598](https://github.com/chawyehsu/hok/commit/d79e5989aa01b1d49cc02003692e2f4b46991ca0))
* **libscoop|event:** added integrity check event and error type ([888afbb](https://github.com/chawyehsu/hok/commit/888afbba203b80dfd4accf57fbc99dc1b348d3e3))
* **libscoop|manifest:** impl Display for License ([e91ff0e](https://github.com/chawyehsu/hok/commit/e91ff0ec48a295a91d771e7256e542e9cab74846))
* **libscoop|resolve:** allow to select installed candidate ([8fb0ec3](https://github.com/chawyehsu/hok/commit/8fb0ec39509128498be1bcbeb3fcddb5edb16838))
* **libscoop|sync:** added SyncOption::EscapeHold for package remove ([ca8fad7](https://github.com/chawyehsu/hok/commit/ca8fad7ffbd1dd1cb0a6d1e03f924e63c5db3364))
* **libscoop:** added package integrity check logic ([57869f7](https://github.com/chawyehsu/hok/commit/57869f763e5a1a9c3668b3028d46787e5ce0e04d))
* **libscoop:** scoop-hash features passthrough ([cb027ce](https://github.com/chawyehsu/hok/commit/cb027cedd98de15aa17602234b824b240c2fcc2c))
* **scoop-hash:** support switching hashing backend ([d38658e](https://github.com/chawyehsu/hok/commit/d38658ef8785df92189b29df7094dadfc609e14c))
* **scoop-hash:** use builder pattern ([87ca347](https://github.com/chawyehsu/hok/commit/87ca3475bd4d5cb947c4ee2702807f944d92c729))


### Bug Fixes

* **hok|list:** only print upgradable when the flag is used ([558d9d3](https://github.com/chawyehsu/hok/commit/558d9d39603d85657986da43f8c98372ac938e30))
* **hok:** accumulate downloaded bytes properly ([d6fabc8](https://github.com/chawyehsu/hok/commit/d6fabc89aa0bfa1428328bddc248c11fe2e9d8e9))
* **libscoop:** package resolving is infallible when OnlyUpgrade is used ([cabd52b](https://github.com/chawyehsu/hok/commit/cabd52bdb1659bb835ad60d9074b8bbdaf345ad0))
* **libscoop:** set install state for package's upgradable reference ([63a54f7](https://github.com/chawyehsu/hok/commit/63a54f7a36ac1cdcb612437d05c752a97ed9a9e3))
* **libscoop:** use upgradable package reference when available ([9dfd93f](https://github.com/chawyehsu/hok/commit/9dfd93fcf58980a113bec1eb781f414c30489de9))


### Performance Improvements

* **libscoop:** 5x speedup on package querying ([90a8815](https://github.com/chawyehsu/hok/commit/90a881550df4c3196cd185ab34e4621f854a41b7))

## [0.1.0-beta.1](https://github.com/chawyehsu/hok/compare/v0.1.0-alpha.3...v0.1.0-beta.1) (2023-07-30)


### ⚠ BREAKING CHANGES

* **libscoop:** Some `Event` variants related to bucekt update progress have been updated to fit the latest codebase.

### Features

* **hok:** support resolving and downloading packages ([bdc08dd](https://github.com/chawyehsu/hok/commit/bdc08dd63898f7af22fa538f20b3fb068e87c26f))
* **libscoop|config:** support `SCOOP_CACHE` and `SCOOP_GLOBAL` envs ([cf2a2a5](https://github.com/chawyehsu/hok/commit/cf2a2a5503c93e5d57b5ac72aec490e2d53b2a7d))
* **libscoop|resolve:** added `resolve_cascade` ([0aa0c52](https://github.com/chawyehsu/hok/commit/0aa0c52802ea2238a31352e9ae0b19c730b7510e))
* **libscoop:** added coordination between `AssumeYes` and `NoDownloadSize` ([5e9d578](https://github.com/chawyehsu/hok/commit/5e9d5784f62fd0eb64009aa23d6d76847c164f46))
* **libscoop:** added support for package resolution and download ([4ff0d95](https://github.com/chawyehsu/hok/commit/4ff0d9573794c003c440477656e808bd527377a2))
* move to v0.1.0-beta.1 ([e1a2376](https://github.com/chawyehsu/hok/commit/e1a2376e58eb91889d7b102aaa6c415cf7b49ef1))


### Bug Fixes

* **libscoop:** ensure ops working dir exist ([0520ae8](https://github.com/chawyehsu/hok/commit/0520ae8fc6e7e560e343a4dffa4c7b514adf92c3))
* **libscoop:** handle wildcard query in upgrade operation ([639e8c6](https://github.com/chawyehsu/hok/commit/639e8c6680f53c34fbd989fdb60a0ea5e9b92c14))
* **libscoop:** update crate categories metadata ([8d6271d](https://github.com/chawyehsu/hok/commit/8d6271d208c40faf2de32787fe9c5ccf32e303f6))
* **libscoop:** update doc comments ([bcd29b4](https://github.com/chawyehsu/hok/commit/bcd29b4b172f7adf5511de457f13ce74ac676370))

## [0.1.0-alpha.3](https://github.com/chawyehsu/hok/compare/v0.1.0-alpha.2...v0.1.0-alpha.3) (2023-07-25)


### ⚠ BREAKING CHANGES

* **libscoop:** `Session::new()` is now infallible.

### Features

* **hok|config:** config-list shows the path ([679c177](https://github.com/chawyehsu/hok/commit/679c1771c036982941bce62e6db55e9098b4e739))
* **libscoop:** impl Default for Session ([d91177a](https://github.com/chawyehsu/hok/commit/d91177a269698b8fbd7b530f0100da82d4ce8879))
* **libscoop:** support loading config from all possible location ([2bcc649](https://github.com/chawyehsu/hok/commit/2bcc649808e8238bef5795c73eab41c182cac61b))
* move to v0.1.0-alpha.3 ([1ecd0ed](https://github.com/chawyehsu/hok/commit/1ecd0edf100ea4a3676494b40b5c72c787ad5501))


### Bug Fixes

* **ci:** remove unneeded condition ([718084f](https://github.com/chawyehsu/hok/commit/718084f80c615513c69a838205e58edd2a553d44))
* **libscoop|fs:** `write_json` should create file instead of dir ([54482a7](https://github.com/chawyehsu/hok/commit/54482a7c8c1733e8d0c01bac5e85fc5da7f4fd3e))
* **libscoop:** fix doctest ([c0237a2](https://github.com/chawyehsu/hok/commit/c0237a2e73d976c4f959bb0928da4cbd0ff3376e))

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
