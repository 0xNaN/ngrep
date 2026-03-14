![ngrep](./assets/ngrep.svg)

<div align="center"> <sup><code>ngrep</code> matching a paragraph from the book <em>Flatland: A Romance of Many Dimensions</em></sup></div>

---

## What is ngrep?

`ngrep` is an experimental tool to help you find text by meaning rather than purely by syntactic matching, with a familiar `grep` interface. The core question is simple: what happens if you add a small bit of semantics to regular expressions via Word Embeddings? `ngrep` explores that idea by extending regex with a new _neural operator_ `~` while keeping the rest of the pattern language intact.

It supports the major regex features you already use (including lookarounds like negative lookahead), so you can combine semantic and literal patterns in one expression. This is made possible by building on [fancy-regex](https://github.com/fancy-regex/fancy-regex), thanks to its robust regex engine.

## The `~` operator

The `~` operator defines a match based on _semantic_ similarity. It finds text that is contextually similar to a given word by leveraging neural [Word Embeddings](https://en.wikipedia.org/wiki/Word_embedding) (yes, 2010s nostalgia).

For example, the expression `~(fruit)+` matches any sequence of characters whose Word Embedding is contextually similar to the Word Embedding of `fruit`:

![fruits](./assets/fruits.svg)

# Syntax and Parameters

You can refine the search using the following syntax:

- `~(word)` e.g: `~(car)`
  Uses the default similarity threshold (from `--threshold` or the model config).
- `~(word;threshold)` e.g: `~(car;0.3)`
  Overrides the threshold for this specific match.
- `~(word1::word2)` e.g: `~(car::bike)`
  Uses the average embedding of `word1` and `word2` as the target.

Note: Since `~(word)` matches only a single model token, you should typically use `~(word)+` to ensure you capture the full word or phrase if the model has split it into multiple parts.)

## Install

From the repository root:

```bash
cargo install --path .
ngrep --help
```

After `ngrep` is installed you have to import some Word Embeddings model to start matching.
Follow these steps to download the English FastText embeddings:

```bash
> curl https://dl.fbaipublicfiles.com/fasttext/vectors-crawl/cc.en.300.vec.gz
> gzip -d cc.en.300.vec.gz
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

## A note on embeddings

Word Embeddings are not reliable or consistent across all inputs. In practice this works best on short paragraphs and concise titles (for example, commit messages), and it is less dependable on isolated single words. Expect to play with the similarity threshold or use multiple semantic matches in the same pattern to get stable results.

## A note on performance

`ngrep`'s current focus is exploration, not performance. It does not preload or cache vectors, performs frequent disk access, and `~` matches are not compiled into standard regex. This is a deliberate choice to keep the implementation simple while the idea is being explored.

To give you a glimpse of the current performance, it takes about 21 seconds to find the most common ways to refer to a big animal in the book _Moby-Dick_ on MacBook Pro M4 (1.2MB of text, 22K lines, English FastText 300d):

```
> ngrep -o '~(big)+ \b~(animal;0.35)+\b' moby.txt | sort | uniq -c | sort -n
> 1 big whale
> 1 enormous creature
> 1 enormous creatures
> 1 gigantic creature
> 1 gigantic fish
> 1 great dromedary
> 1 great hunting
> 1 huge elephant
> 1 huge reptile
> 1 large creatures
> 1 large herd
> 1 large whales
> 1 little cannibal
> 1 small cub
> 1 small fish
> 1 tremendous whale
> 3 great fish
> 3 great monster
> 3 large whale
> 7 great whales
> 8 great whale
```
