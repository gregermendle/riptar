# riptar

Deploy to Cloudflare Workers with [Wrangler](https://developers.cloudflare.com/workers/wrangler/):

```bash
rustup target add wasm32-unknown-unknown
npx wrangler deploy
```

### dither

<img src="https://riptar.gregermendle.com/dither?url=https://riptar.gregermendle.com/riptar/riptar?format=png&height=48&width=48" width="48" height="48" />

basic

```https://riptar.gregermendle.com/dither?url=```

### riptar

<img src="https://riptar.gregermendle.com/riptar/gregermendle" width="48" height="48" />

basic

```https://riptar.gregermendle.com/riptar/:name```

add color

```https://riptar.gregermendle.com/riptar/:name?color=on```

output PNG instead of SVG

```https://riptar.gregermendle.com/riptar/:name?format=png```
