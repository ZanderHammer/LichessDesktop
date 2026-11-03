// Fuzzy matching for the command palette: the query's letters must appear in
// the label in order, not necessarily next to each other ("zi" finds "Zoom
// in"). Case, accents and spaces are ignored.

/** `text` in lower case without accents, as an array of characters. */
function fold(text) {
  return [...text.normalize("NFD").replace(/\p{M}/gu, "").toLocaleLowerCase()];
}

const isWordChar = (char) => /[\p{L}\p{N}]/u.test(char);

const isWordStart = (text, at) => at === 0 || !isWordChar(text[at - 1]);

/**
 * Where `char` matches in `text` from `from` on: right after the previous
 * match if it can, else at the next word start, else anywhere.
 */
function next(text, char, from) {
  if (text[from] === char) return from;
  for (let at = from; at < text.length; at += 1) {
    if (text[at] === char && isWordStart(text, at)) return at;
  }
  return text.indexOf(char, from);
}

/**
 * How well `query` matches `label`, or `null` if it doesn't. Higher is
 * better: matches at the start of words and runs of adjacent letters count
 * most, and a shorter label wins a tie. An empty query matches everything
 * with 0.
 *
 * @param {string} query
 * @param {string} label
 * @returns {number | null}
 */
export function score(query, label) {
  const wanted = fold(query).filter((char) => !/\s/u.test(char));
  if (wanted.length === 0) return 0;
  const text = fold(label);
  let total = 0;
  let previous = -2;
  let from = 0;
  for (const char of wanted) {
    const at = next(text, char, from);
    if (at === -1) return null;
    total += 1;
    if (at === previous + 1) total += 4;
    if (isWordStart(text, at)) total += 6;
    previous = at;
    from = at + 1;
  }
  return total - text.length / 100;
}
