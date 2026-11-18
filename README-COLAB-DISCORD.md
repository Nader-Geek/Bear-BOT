# Bear on Google Colab + Discord

Discord is a control plane only. It receives an image or ZIP, invokes the Rust headless runner, and returns cleaned pages. The runner calls Bear's existing `Pipeline::clean_page` and `export_page`; it does not implement another detector or inpainting engine.

## Current scope
- One image or ZIP chapter per `!clean` command.
- One pipeline instance is held open for all pages in a job.
- `!status` reports worker state.
- Colab is an ephemeral worker; keep the bot token in Colab Secrets.
- The runner currently uses Bear's local automatic ONNX ladder. It does not yet initialize the optional generative sidecar from desktop settings.

## Local Qwen-Image
Bear already contains a local sidecar supporting Qwen-Image-Edit-2511 through SDNQ. Its quantized weights are documented at about 18.16 GB before runtime overhead. The sidecar is separate from the ONNX ladder. The notebook intentionally does not download this large model: check the GPU VRAM and disk first. Headless Qwen activation still needs the same validated sidecar settings that the desktop app supplies; this runner does not claim Qwen is active yet.

The lightest-engine-first strategy already exists: Fill for flat paper, Manga-LaMa for texture/line art, then optional generative reconstruction for difficult regions. We should not add a second classifier/model-selection framework unless real tests show a routing gap.

## Setup
Open `notebooks/Bear_Discord_Colab.ipynb` in Colab, select a GPU runtime, add a Colab Secret named `DISCORD_BOT_TOKEN`, and run the cells in order. The notebook installs Linux build prerequisites, downloads Bear's pinned ONNX weights/runtime, compiles `bear-runner`, and starts the bot.

## Discord setup
Create a bot in the Discord Developer Portal, enable Message Content Intent, and grant permission to read messages/attachments and send messages/files. Never commit the bot token.

## Limits
Colab disconnects and reclaims GPUs; the bot is online only while its notebook session runs. For 24/7 service, move the same bot/runner pair to a persistent GPU host later.
