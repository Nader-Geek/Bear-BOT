# Bear on Google Colab + Discord

Discord is a control plane only. It receives an image or ZIP, invokes the Rust headless runner, and returns cleaned pages. The runner calls Bear's existing `Pipeline::clean_page` and `export_page`; it does not implement another detector or inpainting engine.

## Current scope
- One image or ZIP chapter per `!clean` command.
- One pipeline instance is held open for all pages in a job.
- `!status` reports worker state.
- Colab is an ephemeral worker; keep the bot token in Colab Secrets.
- `!clean` uses Bear's existing automatic ONNX ladder. `!clean qwen` / `!clean-qwen` explicitly selects the local Qwen rung for text outside balloons; speech-bubble text remains on Fill. Qwen is never silently invoked by the automatic command.

## Local Qwen-Image
Bear already contains a local sidecar supporting Qwen-Image-Edit-2511 through SDNQ. Its quantized weights are documented at about 18.16 GB before runtime overhead. The sidecar is separate from the ONNX ladder. The notebook intentionally does not download this large model: check the GPU VRAM and disk first. The notebook includes an optional installer cell that calls Bear's own pinned `sidecar/bootstrap.py` with the exact Qwen checkpoint and LoRAs. After it completes, use `!clean qwen` or `!clean-qwen` with an attached image/ZIP. This is an explicit, slower mode rather than a new automatic classifier.

The default `!clean` path retains the existing lightest-engine-first strategy: Fill for flat paper and Manga-LaMa for texture/line art. Qwen is an opt-in path for complex artwork outside balloons. No extra classifier is introduced, so the design stays simple.

## Setup
Open `notebooks/Bear_Discord_Colab.ipynb` in Colab, select a GPU runtime, add a Colab Secret named `DISCORD_BOT_TOKEN`, and run the cells in order. The notebook installs Linux build prerequisites, downloads Bear's pinned ONNX weights/runtime, compiles `bear-runner`, and starts the bot.

## Discord setup
Create a bot in the Discord Developer Portal, enable Message Content Intent, and grant permission to read messages/attachments and send messages/files. Never commit the bot token.

## Limits
Colab disconnects and reclaims GPUs; the bot is online only while its notebook session runs. For 24/7 service, move the same bot/runner pair to a persistent GPU host later.
