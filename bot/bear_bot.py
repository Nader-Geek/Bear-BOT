import asyncio, os, shutil, tempfile, zipfile
from pathlib import Path
import discord
from discord.ext import commands

TOKEN = os.environ["DISCORD_BOT_TOKEN"]
RUNNER = Path(os.environ.get("BEAR_RUNNER", "./target/release/bear-runner"))
MAX_UPLOAD = int(os.environ.get("BEAR_MAX_UPLOAD_BYTES", str(90 * 1024 * 1024)))
IMAGE_EXTS = {".png", ".jpg", ".jpeg", ".webp", ".tif", ".tiff", ".bmp"}
queue_lock = asyncio.Lock()
worker_status = "idle"
intents = discord.Intents.default()
intents.message_content = True
bot = commands.Bot(command_prefix="!", intents=intents)

def safe_extract(src, dst):
    with zipfile.ZipFile(src) as z:
        for item in z.infolist():
            name = Path(item.filename)
            if name.is_absolute() or ".." in name.parts:
                raise ValueError("Unsafe ZIP path")
            if item.is_dir():
                continue
            target = (dst / name).resolve()
            if dst.resolve() not in target.parents:
                raise ValueError("Unsafe ZIP path")
            target.parent.mkdir(parents=True, exist_ok=True)
            with z.open(item) as inp, target.open("wb") as out:
                shutil.copyfileobj(inp, out)

@bot.command(name="status")
async def status(ctx):
    await ctx.reply("Bear worker: **" + worker_status + "**")

@bot.command(name="clean")
async def clean(ctx):
    global worker_status
    if not ctx.message.attachments:
        await ctx.reply("Attach one image or a ZIP chapter, then send !clean.")
        return
    attachment = ctx.message.attachments[0]
    if attachment.size > MAX_UPLOAD:
        await ctx.reply("File exceeds the configured upload limit.")
        return
    async with queue_lock:
        worker_status = "preparing"
        msg = await ctx.reply("Preparing Bear job…")
        with tempfile.TemporaryDirectory(prefix="bear-job-") as tmp:
            root = Path(tmp)
            incoming = root / Path(attachment.filename).name
            await attachment.save(incoming)
            pages = root / "pages"
            pages.mkdir()
            if incoming.suffix.lower() == ".zip":
                safe_extract(incoming, pages)
            elif incoming.suffix.lower() in IMAGE_EXTS:
                shutil.copy2(incoming, pages / incoming.name)
            else:
                worker_status = "idle"
                await msg.edit(content="Send an image or ZIP file.")
                return
            images = sorted(p for p in pages.rglob("*") if p.is_file() and p.suffix.lower() in IMAGE_EXTS)
            if not images:
                worker_status = "idle"
                await msg.edit(content="No supported page images found.")
                return
            flat = root / "input"
            flat.mkdir()
            for i, p in enumerate(images):
                shutil.copy2(p, flat / (str(i + 1).zfill(4) + p.suffix.lower()))
            output = root / "output"
            worker_status = "cleaning " + str(len(images)) + " page(s)"
            proc = await asyncio.create_subprocess_exec(str(RUNNER), str(flat), str(output), stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.STDOUT)
            last = ""
            while True:
                line = await proc.stdout.readline()
                if not line:
                    break
                value = line.decode(errors="replace").strip()
                if value.startswith("PROGRESS ") or value.startswith("DONE "):
                    last = value
                    await msg.edit(content="Bear: `" + value + "`")
            code = await proc.wait()
            if code:
                worker_status = "failed"
                await msg.edit(content="Runner failed. Check the Colab output for details.")
                return
            result = root / "bear-cleaned.zip"
            with zipfile.ZipFile(result, "w", zipfile.ZIP_DEFLATED) as z:
                for p in sorted(output.glob("*.png")):
                    z.write(p, p.name)
            await ctx.reply(file=discord.File(result, filename="bear-cleaned.zip"))
            worker_status = "idle"
            await msg.edit(content="Completed. " + last)

@bot.command(name="help")
async def help_command(ctx):
    await ctx.reply("Commands: !clean (attach image/ZIP), !status.")

bot.run(TOKEN)
