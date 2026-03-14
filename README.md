![ngrep](./assets/ngrep.svg)

<div align="center"> <sup><em>ngrep</em> matching a paragraph from the book <em>Flatland: A Romance of Many Dimensions</em></sup></div>

---

`ngrep` is an experimental tool that lets you find text by meaning rather than by syntactic matching. The core question is simple: _what happens when regular expressions are enriched with a small bit of semantics via Word Embeddings?_

`ngrep` explores that idea with a familiar grep interface, extending regex with a new neural operator `~` while keeping the rest of the language intact. It supports the major regex features you already use (including lookarounds like negative lookahead), so you can combine semantic and literal patterns in one expression, built on top of the fantastic [🦀 fancy-regex](https://github.com/fancy-regex/fancy-regex).

## The `~` operator

The `~` operator matches by _semantic_ similarity, using neural [Word Embeddings](https://en.wikipedia.org/wiki/Word_embedding) (yes, 2010s nostalgia).
For example, `~(fruit)+` matches any token whose embedding is close to that of fruit:

![fruits](./assets/fruits.svg)

# Syntax and Parameters

You can refine the search using the following syntax:

- `~(word)` e.g: `~(car)`
  Match word using the default similarity threshold (from `--threshold` or the model config).
- `~(word;threshold)` e.g: `~(car;0.3)`
  Overrides the threshold for this specific match.
- `~(word1::word2)` e.g: `~(car::bike)`
  Uses the average embedding of `word1` and `word2` as the target.

* `~()` matches only a single token, use `~()+` to capture full words or phrases
* Word Embeddings are inherently imprecise, expect to tune the threshold and combine multiple `~` operators to get stable results

## Install

From the repository root:

```bash
cargo install --path .
ngrep --help
```

After `ngrep` is installed you have to import some Word Embeddings model to start matching.
Follow these steps to download the English FastText embeddings:

```bash
curl https://dl.fbaipublicfiles.com/fasttext/vectors-crawl/cc.en.300.vec.gz
gzip -d cc.en.300.vec.gz
```

Then import and use them:

```bash
ngrep import cc.en.300.vec ften
echo 'hello world' | ngrep '~(hey)+ ~(planet)+'
```

Alternatively you can import any embeddings in the `txt` format and configure the default model with `ngrep config`. You can import multiple models and switch between them with `ngrep config` or `--model`, but only one model is used at a time for matching:

- [FastText Word vectors for 157 languages](https://fasttext.cc/docs/en/crawl-vectors.html#models)
- [Wikipedia2Vec with ENTITY vectors](https://wikipedia2vec.github.io/wikipedia2vec/pretrained/)
- [GloVe: Global Vectors for Word Representation](https://nlp.stanford.edu/projects/glove/)

## A note on performance

`ngrep`'s current focus is exploration, not performance. It does not preload or cache vectors, performs frequent disk access, and `~` matches are not compiled into standard regex. This is a deliberate choice to keep the implementation simple while the idea is being explored.

To give you a glimpse of the current performance, it takes about ~18s to find the most common ways to refer to a big animal in the book _Moby-Dick_ on MacBook Pro M4 (1.2MB of text, 22K lines, English FastText 300d):

```
wget -q https://raw.githubusercontent.com/massimo-nazaria/bash-textgen/refs/heads/main/moby-dick.txt
time time ngrep -o '~(big)+ \b~(animal;0.35)+\b' moby-dick.txt | sort | uniq -c | sort -n
   1 big whale
   1 enormous creature
   1 enormous creatures
   1 gigantic creature
   1 gigantic fish
   1 great dromedary
   1 great hunting
   1 huge elephant
   1 huge reptile
   1 large herd
   1 little cannibal
   1 small cub
   1 small fish
   1 tremendous whale
   2 great fish
   3 great monster
   3 large whale
   5 great whales
   7 great whale

real	0m17.291s
user	0m16.577s
sys	  0m0.726s
```
