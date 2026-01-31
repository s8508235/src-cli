## Overview
See [How Fast Can You Read? - Speed Reading Challenge](https://www.youtube.com/watch?v=NdKcDPBQ-Lw).


## Usage 
Convert text to video using FFmpeg.

### Colors
- Named colors (white, black, red, etc.)
- Hex colors (#FF0000, 0xFF0000)
- RGB format (rgb(255,0,0))

## Example
```sh
# From argument
src-cli --text "Hello, World!"

# From pipe
echo "Hello!" | src-cli -o output.mp4

# Custom styling
src-cli -t "Big News" --text-color yellow --bg-color blue
```

## Claude
It used concat files in the beginning and after providing stackoverflow links, 
it created temp images to achive. Finally after providing [Ultimate guide to ffmpeg](https://img.ly/blog/ultimate-guide-to-ffmpeg/#ffmpeg-history), it uses filter to do the job. (Sonnect 4.5)

What changed:

Single FFmpeg command - no intermediate files!
Uses enable='between(t,start,end)' - controls when each word appears based on timestamp
Chains multiple drawtext filters - one per word, each with its own time window
Much faster and cleaner - all done in one pass

How it works:

Creates a black video of total duration
Each word has a drawtext filter that's only enabled during its time window
At 300 WPM: "Hello World" → "Hello" shows 0.0-0.2s, "World" shows 0.2-0.4s

## TODO
- find stress character for words

## MISC
- install Google Noto
- for bgm [DOVA-SYNDROME YouTube Official ](https://www.youtube.com/@DOVASYNDROMEYouTubeOfficial) is a good place.