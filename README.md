## Realtime Video Encoding Benchmark Tool

A command-line tool wrapper around ffmpeg that provides:

- reliable way to benchmark real-time video encoding capabilities of your hardware
- determination of encoder-specific settings that your hardware can handle
- optional accurate video encode quality scoring via <a href="https://github.com/Netflix/vmaf">vmaf</a>
- no storage overhead during benchmarking due to some clever use of data streaming via TCP
- bitrate increasing permutations to find minimum bitrate needed for visually lossless game streaming
- a clean report at the end of the permutation results to help guide hardware decisions & streaming guides

```shell
# sample output for encoding a 4K@60 input file
[Permutation 2/21]
[ETR: 9m20s]
[Bitrate: 10Mb/s]
[-preset p1 -tune ll -profile:v high -rc cbr -cbr true]
Running benchmark to test whether encoder can keep up with input...
  [00:00:25] [####################################] 1923/1923 frames (00:00:00)
  Average FPS:  78
  1%'ile:       68
  90%'ile:      94
```

For a quick-start guide for common use-cases, see <b>Quick-Run Guide</b>.

For a quick reference of what to expect for your hardware, or what bitrate to configure in Moonlight when doing at home
game streaming, see the <b>Expected Performance</b> section at the bottom of the readme.

---

## Minimum system specs suggested

In addition to the mentioned specs below, you should _not_ have drive compression enabled for Windows on the drive you
plan to store the files on. This can heavily limit sequential read speeds and affect the results of the tool.

```text
OS: Windows, Mac or Linux
Processor: CPU with at least 6 cores
GPU: GPU w/ hardware encoder, in the main x16 PCI slot of your PC (for max PCI bandwidth)
Memory: >= 8GB RAM (higher is always better)
Storage Space: 120GB minimum (all benchmark files take up about 120GB)
Storage Type/Speed: 
    -if benchmarking <= 1080@120, any SATA ssd will work
    -if benchmarking >= 1080@120, you MUST use an nvme drive with > 1GB/s sequential read speeds
```

The more cores you have, the faster the quality scoring via vmaf will happen, and the faster the benchmarks will
run overall. Plus, your encoder of choice might be limited based on your CPU itself.


---

## Installation & Setup requirements

Note: tool has been tested with ffmpeg version `5.1.2`, so it's highly suggested to use the same version, or at least
version `5.*` of ffmpeg/ffprobe.

1) Installation of <a href='https://ffmpeg.org/download.html'>ffmpeg</a>

    - For Windows, recommend downloading the binaries for Windows from <a href='https://www.gyan.dev/ffmpeg/builds/'>
      gyan.dev</a>, specifically the `ffmpeg-release-full` one, which should include all needed features and tools
    - For Mac/Linux, install both `ffmpeg` and `ffprobe` at mentioned versions above
2) <a href='https://www.7-zip.org/download.html'>7-Zip</a> to unzip any downloaded ffmpeg binaries
3) ffmpeg/ffprobe must be available on your path (tool will error out if it can't find
   either); <a href='https://www.architectryan.com/2018/03/17/add-to-the-path-on-windows-10/'>quick path setup guide for
   Windows
   10+</a>
4) Download the built executable for your platform from the release section of this repo onto the SSD that you wish to
   run the benchmark on
5) Download all the raw video source files onto the same SSD where the executable is (and in the same folder)
   from <a href='https://www.dropbox.com/sh/x08pkk47lc1v5ex/AADGaoOjOcA0-uPo7I0NaxL-a?dl=0'>here</a>. If you only wish
   to
   benchmark one specific resolution, only download that file and make sure to specify it as noted in the **Run using
   the executable** section below

(Note: due to the tool sending encoded video over your local network to test quality, you will not need more storage
space than just what is needed to hold the original source video files).

---

## Running the tool

Keep in mind, this tool is going to stress the encoder of your choice. It's highly recommended when running this tool
to <i>not use your computer for any other tasks</i> as that can affect the results.

i.e. if you're using your GPU for encoding, it's best to not have any programs open that could use your GPU.

#### Run using executable

For a complete list of all command line arguments, run:

`./ebt -h`

Typical usage would be:

```shell
# general example, note that executable name might be different
./ebt -e <encoder_name> -b <target_bitrate> -s <input_file.y4m>

# specific example, wanting to encode H264 using your Nvidia GPU at a 10Mb/s bitrate target
./ebt -e h264_nvenc -b 10 -s 1080-60.y4m
```

#### Run using cargo

(See **Contributing** section for version of Rust to install)

`cargo run --release -- -e h264_nvenc -b 10 -s 1080-60.y4m`

#### Stopping the benchmark

Kill the benchmark by hitting `ctrl-c` in the terminal/console where the benchmark is running.

---

## Quick-Run Guide

This tool can do a lot of things, however it's likely that your use case is very specific. Here are the typical use
cases & commands that you'd use.

**Note:** the input file that you provide is what determines the resolution you'll be benchmarking.

### The Benchmark: Run Pre-Configured Encodes to Compare to Others

This feature of the tool is likely going to be the most popular, as it's intended to compare different systems
encoding performances in a standardized way.

Note: you <i>must</i> download all the video source files to be able to run the standard benchmark. See the <b>
Installation & Setup Requirements</b> section.

It does the following:

- runs pre-configured encoder settings & bitrate values chosen by the author
- runs benchmark on all supported resolutions, outputting fps results to the console & a `benchmark.log` file

Simple run the following (and choose your encoder):

`./ebc -e h264_nvenc`

If you so chose, you can also calculate the quality of the encode as well, although at the target configurations &
bitrates you should expect a vmaf score of 95:

`./ebc -e h264_nvenc -c`

You may want the benchmark to stop early if it can't encode at the file's target fps; if so, use the `-d` parameter to
detect encoder overload and stop the benchmark:

`./ebc -e h264_nvenc -d`

**Location of Encoder Setting Used**: to see the chosen encoder preset for this benchmark, see the respective encoder's
implementation of `run_standard_only()` method, found in `src/permutations`. These tend to lean on the side of a lower
preset to help keep fps statistics a bit higher (since you can always use more bitrate to improve quality).

**Location of Bitrate Used:** you can also find the pre-configured bitrate value for that encoder within the encoder's
implementation of `get_resolution_to_bitrate_map()`, found in `src/permutations`. These values are typically the
bitrates for the 120fps findings.

### Running benchmark over encoder settings

This will run through all permutations of encoder settings, for your given resolution input file, and produce fps
statistics, at the default bitrate of 10Mb/s. This gives you a good idea of the performance you can expect for your
encoder.

`./ebt -e h264_nvenc -s 4k-60.y4m`

You may want to gather fps statistics at higher bitrates that you're more likely to stream at, since output bitrate can
affect encoder performance. Do this by using the following:

`./ebt -e h264_nvenc -s 4k-60.y4m -b 50`

If you are curious on how _the minimum bitrate required_ can be identified, this tool can help you find that as well in
the next section.

### Bitrate & encoder settings combinations to achieve visually lossless game streaming

When using Moonlight as your game streaming client, it auto-recommends a bitrate for you to stream at. Most of the time
this is pretty accurate for lower resolutions, however depending on your hardware's capabilities you might be able to
get away with less bitrate than it suggests.

For example: Moonlight auto-selects `80Mb/s` for streaming 4K@60 game content. However from our testing, you really only
need `45-50Mb/s` when encoding using H264_NVENC.

To simply test what quality scores you can get at a given bitrate, use the following:

`./ebt -e h264_nvenc -s 4k-60.y4m -c -b 10`

If you're not sure on what bitrate would achieve visually lossless quality, provide a starting bitrate & max bitrate to
permute over (in 5Mb/s intervals). In the below example, every encoder setting will be tested at
**[10, 15, 20, ..., 100 Mb/s]** (until vmaf score of 95 is achieved):

`./ebt -e h264_nvenc -s 4k-60.y4m -c -b 10 -m 100`

When the tool detects that you've hit a `95` vmaf score, it will stop permuting. In the above example, the tool would
stop permuting once it gets to `50Mb/s` since we know that's the point where you get visually lossless 4K@60 with
H264_NVENC, and any higher amount of bitrate does not significantly improve quality and can actually reduce encoder
performance.

---

## Discussion

### What this benchmark can provide for you

1) Determine what resolution/fps your specific system can encode in real-time with less guess work
2) Permute over configured encoder settings to identify what settings maximize encoder real-time average fps
3) Make use of quality scores to determine encoder setting & bitrate combinations that provides visually lossless
   quality
4) Provide encoder settings in a copy-paste format, making it easy to apply them into OBS Studio/Sunshine

### How to Interpret FPS Statistics

The tool will provide you with the following FPS stats when running through permutations:

```text
  Average FPS:  78
  1%'ile:       68
  90%'ile:      94
```

Each stat has specific things that it can tell you about what your system can do in real-time.

- _Average:_ gives an idea of the overall experience you can expect during encoding
- _1%'ile:_ gives an indication of detected dips/low points in framerate
- _90%'ile:_ gives an indication of the upper-limits of the encoding capabilities of your hardware

When choosing encoder permutation settings to use, you should look at all 3 datapoints before deciding what your system
can handle.

#### A Good Encoding Experience Example

Let's say you get the following fps stats:

```text
  Average FPS:  78
  1%'ile:       68
  90%'ile:      94
```

You have an average of `78fps` and a 1%'ile of `68fps`, I can confidently say that
my system will produce a smooth and consistent `60fps` encode experience.

However if I really wanted to, I could look at the 90%'ile and know that my system _could_ periodically do `90fps` but
will drop down to as low as `68fps`. If that fps variance is fine with you, you can feel free to set your target fps
to `90fps`. Just know that at heavier encode times or game content where there's more movement/variance, your encode fps
will drop.

#### A Bad Encoding Experience Example

Let's say instead, you see the following stats:

```text
  Average FPS:  78
  1%'ile:       30
  90%'ile:      85
```

Notice the 1%'ile `30fps` is much lower than the average and the 90%'ile. In this case, you may end up seeing fps drops
during encoding that are drastic and would most likely provide a bad encoding experience.

For results like these, it's recommended to try using different hardware/encoding settings to have the 1%'ile be much
closer to the average. Or, just know that you might see some hard dips in FPS and consider not game streaming at all for
a better experience.

### Real-time encoding terminology

The benchmark is always checking whether your encoder, at a given resolution/bitrate/fps, can keep up with <i>at least
the given fps</i>. By default however, the actual encoding speed may be much higher than the input file. This is to help
cut down on runtime during benchmarking.

**Disclaimers**

_It is possible that when you go to apply these settings in OBS Studio, or Sunshine's game stream hosting software,
that the encoder/ffmpeg version being used there may perform different, i.e. most likely worse than this benchmarking
tool._

_This tool only tells you whether your host system can encode the video at the given parameters; it does not tell you
whether the client you intend to stream <b>TO</b> can decode at the same speed. You may find that your client device
cannot decode the incoming video as fast as it's sent._

_You may find that what works on your machine, does not work on another with similar hardware. This is expected, and is
why you should run the tool on your machine to get specific results to your setup._

### Visually Lossless Terminology

Any encoded video that scores >= 95 vmaf score is considered visually lossless, and is what you are shooting for in
terms of encoded video quality. Anything lower and you end up seeing minor blockiness or artifacting.

By default, the tool does not calculate vmaf score to initially focus on producing fps statistics. However, you can have
the tool calculate vmaf score on each permutation by using the `-c` flag.

Using `-c` in combination with `--

### Skipping Duplicate Scoring Permutations by Default

By default, the benchmark will detect if encoder settings produce the same vmaf score as previously calculated ones.
This is done during the initial pass of all encoder setting permutations for a given bitrate.

Each subsequence bitrate & encoder settings permutation will effectively ignore duplicated encoder settings that produce
duplicated vmaf results in an effort to cut down on pointless calculation time.

If you so desire, you can still have those permutations run and end up in the normal produced report by specifying
the `-a` option.

Note: a footnote to the results file will be added of what encoder settings produced similar results for your reference
later.

### Detecting video encoder overload & skipping by default

If you've streamed using OBS Studio before, you might have seen:

![img.png](docs/obs-encoder-overload.png)

This tool has logic in it to detect encoder overload as well, by keeping an eye on the <i>current fps</i> during
encoding. For example, if the target fps is `60`, this tool will attempt to encode the source video file at
exactly `60fps`.

If at any point the tool detects that the current fps is less than the target, the tool will stop and indicate that an
encoder overload has occurred. If you still wish to let these encodes keep going (to see full fps statistics), you can
do this by providing the `-i` option to ignore when the encoder gets overloaded.

Note: in the stats output file, any encoder/bitrate/fps permutation that results in an overloaded encoder will have
a `[O]` at the beginning of it's result stats.

### Source input files

All source files for this tool are captured with OBS Studio as `yuv4` raw video with `4:2:0` chroma subsampling, at
specific resolutions/frame rates.
Raw video at this chroma subsample is the closest you can get to simulating encoding a game that you're playing in
real-time, much like you would
when streaming to Twitch/Youtube. Any higher chroma subsample will produce lower scoring/performing results.

Input source files are real gameplay captures. The two recorded FPS rates is 60 and 120, however the benchmark tool can
encode at much higher rates if your hardware can support that.

You can make your own conclusions about > 120 fps or < 60 fps bitrates that you'll need, since it's typically a linear
relationship. For example:

```text
// these have been calculated as what you'd need for visually lossless quality
// notice how the 120fps requires twice the bitrate
720@60  H264 -> 10Mb/s
720@120 H264 -> 20Mb/s

// you can guess that lower fps, and higher fps, will have a bitrate that scales accordingly:
720@30  H264 -> 5Mb/s
720@240 H264 -> 40Mb/s
```

_Note: if you play a different genre of game, or you have overlays in your OBS studio setup, your encoding performance
may vary. It is difficult/impossible to cover all possible inputs when benchmarking video encoding._

### Why results from one machine might not apply to another

It can be difficult to determine <i>what</i> specific encoder settings would work best on your system without a lot of
trial and error. A few of the factors that can affect this:

- GPU's encoder generation/hardware
- overall system's performance (combination of motherboard, CPU, and other factors such as storage/RAM)
- driver versions being used to encode the video files

With this in mind, you may find that what works really well on your system, may not work the same on another's (even
with the same GPU hardware). This is why it's important to run the benchmark on your own system for specific
settings/capabilities of your entire setup.

### Higher bitrate always increases vmaf score

This is expected. By convention, using a low encoder preset/tune combination but allowing more data to be sent per
second, will mean a higher quality stream.

This tool will help you find at what bitrate you reach your max achievable quality at given encoder settings, but if you
can afford it you can always increase the bitrate above that (with diminishing quality returns). This is almost always
the case with at-home game streaming where you're less bandwidth limited.

Game streaming <i>outside your network</i> or over cellular is where you'll truly become bandwidth limited and where
this tool can be useful.

### FPS Statistics Do Not Change Much with Higher Bitrates

This is expected. The bitrate value is simply how much data to transmit _after the encode has happened_, and typically
does not affect how _fast_
your encoder can encode an input.

The tool defaults to not permuting over bitrate values due to this fact. The only reason you'd want to also permute over
bitrates or try higher values, is to get a higher vmaf score or to achieve visually lossless video quality.

### Number of threads used by VMAF in tool

The tool automatically chooses the maximum available threads on your machine (including hyper-threads). This ensures
maximum performance when doing vmaf calculations.

There are diminishing returns the more threads you throw at VMAF, but these small gains will make a huge difference when
running through encoder permutations/benchmarks.

<b>Note:</b> threads higher than your <i>physical core count</i> don't add any extra performance and are effectively
unused. The tool might be more robust in the future to calculate 1 thread per physical core but, there's no issue with
just specifying all threads.

### Use of 'n_subsample' for calculation speedup in VMAF

The tool uses a value of the parameter `n_subsample=5` passed to VMAF, to cut down the vmaf calculation to almost half.
This effectively tells VMAF to look every 5th frame when doing the score calculation.

A <i>negligible</i> score difference of < 1 score point was observed when running through both low fps and high-fps
video content, with and without the use of `n_subsample`.

Higher values than 5, at least for the 30 second video inputs began to show >= 2 score point differences and provided no
performance benefit.

---

## Encoder Specific Notes

### H264/HEVC NVENC

#### Presets & Tunes

Only the latest presets, i.e. `[p1-p7]` are being used, along with the following tunes: `[ll, ull, hq]` when testing
encoder permutations. All other legacy presets end up mapping to a combination of the mentioned presets/tunes and just
adds extra computation time.

The use of the `lossless` tune or `lossless` preset effectively ignores the bitrate that you set, so these are not
included in permutations, since we're focused on specific/replicable bitrate targets with vmaf scores.

#### Profiles

It was determined that use of any other profile than `high` did not improve results; i.e. either lowered the vmaf score,
or did not increase the average fps. Thus, the tool is sticking to using 'high' for H264, and 'main' for HEVC.

Documentation used for decisions made when using this codec:

- <a href='https://docs.nvidia.com/video-technologies/video-codec-sdk/nvenc-preset-migration-guide/#h264-preset-migration-table'>
  Nvenc Preset Migration Guide</a>
- <a href='https://docs.nvidia.com/video-technologies/video-codec-sdk/nvenc-video-encoder-api-prog-guide/index.html#encoder-tuning-info-and-preset-configurations'>
  Nvidia's Video Codec SDK Documentation</a>

---

## Expected Performance

While building this tool, the author used specific GPU's for validation. Along the way, he was able to find the maximum
limit of the hardware he has available to him, as well as minimum bitrates needed to achieve visually lossless results.

Hopefully performance details listed below will help you identify where your specific hardware lies relative to the
author's own system.

### Minimum Spec'd PC

- <b>CPU:</b> <a href='## Issues, Bugs & Feature Requests'>Intel i5-8400 (6 cores/6 threads)</a>
- <b>RAM:</b> 16GB of <a href='https://a.co/d/0O1qryh'>G.Skill Ripjaws V DDR4 3200Mhz</a>
- <b>GPU:</b> <a href='https://a.co/d/iJLdgKx'>Asus GTX 1660 Super</a>
- <b>NVME SSD</b>: <a href='https://a.co/d/clUM7ta'>PNY250GB NVMe PCI Gen3 x4</a>
- <b>NVENC Arch & Gen:</b> Turing, 7th Gen
- <b>Nvidia Driver:</b> 527.56

The following are the minimum bitrates you'd need to achieve visually lossless results on the above Turing GPU:

Note: HEVC is about 30% more efficient but that doesn't appear to make that huge of a difference in your ending bitrate.

```text
NVENC H264: 720@60   -> 10Mb/s
NVENC HEVC: 720@60   -> 5-10Mb/s

NVENC H264: 720@120  -> 25Mb/s
NVENC HEVC: 720@120  -> 20-25Mb/s

NVENC H264: 1080@60  -> 20Mb/s
NVENC HEVC: 1080@60  -> 15-20Mb/s

NVENC H264: 1080@120 -> 40Mb/s
NVENC HEVC: 1080@120 -> 35-40Mb/s

NVENC H264: 2K@60    -> 20-25Mb/s
NVENC HEVC: 2K@60    -> 20-25Mb/s

NVENC H264: 2K@120    -> 50-55Mb/s
NVENC HEVC: 2K@120    -> 50Mb/s

NVENC H264: 4K@60    -> 45-50Mb/s
NVENC HEVC: 4K@60    -> 40-45Mb/s

// Note: Turing GPU used could not get any higher than 4K@75-90 average fps
// however, most likely the bitrate required for visually lossless 4K@120 is ~100Mb/s

NVENC H264: 4K@120   -> ???
NVENC HEVC: 4K@120   -> ???
```

Notice that with HEVC you can achieve the same level of visually lossless quality with slightly less bitrate, at least
with NVENC. This is going to be the case with an AV1 encoder on newer GPU's. When at all possible, it's suggested to use
the encoder that provides higher quality at a lower bitrate.
---

## Issues, Bugs & Feature Requests

Feel free to open issues in the repository if you find issues, and we'll try to get around to fixing them or
implementing the feature requests.

Screenshots or log file uploads are much appreciated!

---

## Contributing

Project is written in Rust, with version `1.66.0` at time of writing.

Setup your dev environment for Rust and you'll be able to contribute.