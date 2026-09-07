import test from "node:test";
import assert from "node:assert/strict";
import React from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { renderToStaticMarkup } from "react-dom/server";
import { normalizeMarkdownLinkDestinations } from "../src/markdownLinks.ts";

test("renders a Markdown link when its label and destination span source lines", () => {
  const markdown = "[the link label\ncontinues](https://example.com/docs/\n  architecture)";

  const rendered = renderToStaticMarkup(
    React.createElement(
      ReactMarkdown,
      { remarkPlugins: [remarkGfm] },
      normalizeMarkdownLinkDestinations(markdown),
    ),
  );

  assert.equal(
    rendered,
    "<p><a href=\"https://example.com/docs/architecture\">the link label\ncontinues</a></p>",
  );
});

test("does not join line breaks in ordinary text, inline code, or fenced code", () => {
  const markdown = [
    "ordinary text ](https://example.com/",
    "not-a-link)",
    "",
    "`inline ](https://example.com/`",
    "continued)`",
    "",
    "```md",
    "[code](https://example.com/",
    "code)",
    "```",
    "",
    "    [indented code](https://example.com/",
    "    code)",
  ].join("\n");

  assert.equal(normalizeMarkdownLinkDestinations(markdown), markdown);
});

test("keeps link titles and balanced parentheses intact while joining destinations", () => {
  const markdown = [
    "[title](https://example.com/docs/",
    "  architecture",
    "  \"A title\")",
    "[parenthesized](https://example.com/a(",
    "  b))",
  ].join("\n");

  assert.equal(
    normalizeMarkdownLinkDestinations(markdown),
    "[title](https://example.com/docs/architecture \"A title\")\n[parenthesized](https://example.com/a(b))",
  );
});

test("does not cross a blank line while looking for a multiline destination", () => {
  const markdown = "[not a link](https://example.com/\n\ncontinued)";

  assert.equal(normalizeMarkdownLinkDestinations(markdown), markdown);
});

test("preserves whitespace inside multiline link titles", () => {
  const markdown = "[title](https://example.com/\n  \"A long\n  title\")";

  assert.equal(
    normalizeMarkdownLinkDestinations(markdown),
    "[title](https://example.com/ \"A long title\")",
  );
});

test("does not treat a fenced code content line with an info string as a closer", () => {
  const markdown = [
    "```md",
    "```sh",
    "[code](https://example.com/",
    "  wrapped)",
    "```",
    "",
    "[real](https://example.com/",
    "  wrapped)",
  ].join("\n");

  assert.equal(
    normalizeMarkdownLinkDestinations(markdown),
    markdown.slice(0, markdown.indexOf("[real]")) + "[real](https://example.com/wrapped)",
  );
});

test("does not normalize a destination-like sequence after an inline-code label", () => {
  const markdown = "`[placeholder` then ](docs/\n  path)";

  assert.equal(normalizeMarkdownLinkDestinations(markdown), markdown);
});
