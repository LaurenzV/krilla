# Subsetting

The emoji fonts were subsetted using the following commands:

- Noto Color Emoji COLR: `hb-subset --text-file ../emojis.txt --drop-tables=SVG --output-file=NotoColorEmoji.COLR.subset.ttf NotoColorEmoji-Regular.ttf`
- Noto Color EMOJI EBDT: `hb-subset --text-file ../emojis.txt --output-file=NotoColorEmoji.CBDT.subset.ttf NotoColorEmoji.ttf`
- Twitter Color Emoji: `fonttools subset --text-file=../emojis.txt --output-file=TwitterColorEmoji.subset.ttf TwitterColorEmoji-SVGinOT.ttf`
- Emoji One: `fonttools subset --text-file=../emojis.txt --output-file=EmojiOne.subset.otf EmojiOneColor.otf`