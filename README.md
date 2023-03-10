Tech demo for potential future Legion Prof interface.

### TODO

- [x] Check nested viewport culling
- [x] Slot items by row
- [x] Row check for hover/click
- [x] Better explanatory text
- [x] Utilization plots
- [x] Vertical cursor
- [x] Node selection
- [x] Expand all of a kind (cpu/gpu/etc)Rects on 1-row proc show up at top
- [x] Stop hardcoding kinds
- [x] Multiple profiles
- [x] There is a bug when you move the cursor near the right edge of the screen, the scroll bar gets pushed away
- [x] Timestamps on the vertical cursor
- [x] Horizontal zoom
- [x] Fetch from data source
- [x] Bug in single-row slots not rendered at bottom
- [x] Render data in tiles
- [ ] Long-running tasks that cross tile boundary
- [ ] Asynchronous data fetch
- [ ] Horizontal pan (including drag, keyboard, horizontal scroll wheel)
- [ ] Vertical zoom
- [ ] Search (with load all data option to get better search results)
- [ ] Task detail view
- [ ] Keyboard bindings (e.g., arrow keys to select panels, space bar to toggle expand/collapse)

### Native

```
cargo run --release
```

Ubuntu dependencies:

```
sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libspeechd-dev libxkbcommon-dev libssl-dev
```

Fedora Rawhide dependencies:

```
dnf install clang clang-devel clang-tools-extra speech-dispatcher-devel libxkbcommon-devel pkg-config openssl-devel libxcb-devel fontconfig-devel
```

### Web Locally

You can compile your app to [WASM](https://en.wikipedia.org/wiki/WebAssembly) and publish it as a web page.

We use [Trunk](https://trunkrs.dev/) to build for web target.
1. Install Trunk with `cargo install --locked trunk`.
2. Run `trunk serve` to build and serve on `http://127.0.0.1:8080`. Trunk will rebuild automatically if you edit the project.
3. Open `http://127.0.0.1:8080/index.html#dev` in a browser. See the warning below.

> `assets/sw.js` script will try to cache our app, and loads the cached version when it cannot connect to server allowing your app to work offline (like PWA).
> appending `#dev` to `index.html` will skip this caching, allowing us to load the latest builds during development.

### Web Deploy

1. Just run `trunk build --release`.
2. It will generate a `dist` directory as a "static html" website
3. Upload the `dist` directory to any of the numerous free hosting websites including [GitHub Pages](https://docs.github.com/en/free-pro-team@latest/github/working-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site).
4. we already provide a workflow that auto-deploys our app to GitHub pages if you enable it.
> To enable Github Pages, you need to go to Repository -> Settings -> Pages -> Source -> set to `gh-pages` branch and `/` (root).
>
> If `gh-pages` is not available in `Source`, just create and push a branch called `gh-pages` and it should be available.

You can test the template app at <https://elliottslaughter.github.io/test-egui>.
