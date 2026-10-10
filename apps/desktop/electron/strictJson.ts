const MAX_DEPTH = 32;

class JsonScanner {
  private index = 0;
  private readonly source: string;

  constructor(source: string) {
    this.source = source;
  }

  scan(): void {
    this.skipWhitespace();
    this.value(0);
    this.skipWhitespace();
    if (this.index !== this.source.length) this.fail('trailing data');
  }

  private value(depth: number): void {
    this.skipWhitespace();
    const char = this.source[this.index];
    if (char === '{') return this.object(depth + 1);
    if (char === '[') return this.array(depth + 1);
    if (char === '"') {
      this.string();
      return;
    }
    if (char === 't' && this.literal('true')) return;
    if (char === 'f' && this.literal('false')) return;
    if (char === 'n' && this.literal('null')) return;
    if (char !== undefined && char >= '0' && char <= '9') {
      this.number();
      return;
    }
    this.fail('invalid value');
  }

  private object(depth: number): void {
    this.checkDepth(depth);
    this.index++;
    this.skipWhitespace();
    if (this.take('}')) return;

    const keys = new Set<string>();
    while (true) {
      this.skipWhitespace();
      if (this.source[this.index] !== '"') this.fail('object key must be a string');
      const key = this.string();
      if (keys.has(key)) this.fail(`duplicate object key ${JSON.stringify(key)}`);
      keys.add(key);
      this.skipWhitespace();
      if (!this.take(':')) this.fail('expected colon after object key');
      this.value(depth);
      this.skipWhitespace();
      if (this.take('}')) return;
      if (!this.take(',')) this.fail('expected comma between object fields');
    }
  }

  private array(depth: number): void {
    this.checkDepth(depth);
    this.index++;
    this.skipWhitespace();
    if (this.take(']')) return;
    while (true) {
      this.value(depth);
      this.skipWhitespace();
      if (this.take(']')) return;
      if (!this.take(',')) this.fail('expected comma between array items');
    }
  }

  private string(): string {
    const start = this.index++;
    while (this.index < this.source.length) {
      const code = this.source.charCodeAt(this.index);
      if (code === 0x22) {
        this.index++;
        return JSON.parse(this.source.slice(start, this.index)) as string;
      }
      if (code < 0x20) this.fail('unescaped control character in string');
      if (code !== 0x5c) {
        this.index++;
        continue;
      }

      this.index++;
      const escape = this.source[this.index];
      if ('"\\/bfnrt'.includes(escape ?? '')) {
        this.index++;
        continue;
      }
      if (escape !== 'u') this.fail('invalid string escape');
      const codeUnit = this.readHexCodeUnit();
      if (codeUnit >= 0xd800 && codeUnit <= 0xdbff) {
        if (this.source.slice(this.index, this.index + 2) !== '\\u') this.fail('unpaired high surrogate');
        this.index += 2;
        const low = this.readHexCodeUnit();
        if (low < 0xdc00 || low > 0xdfff) this.fail('unpaired high surrogate');
      } else if (codeUnit >= 0xdc00 && codeUnit <= 0xdfff) {
        this.fail('unpaired low surrogate');
      }
    }
    this.fail('unterminated string');
  }

  private readHexCodeUnit(): number {
    const hexStart = this.index + 1;
    const hex = this.source.slice(hexStart, hexStart + 4);
    if (hex.length !== 4 || !/^[0-9a-fA-F]{4}$/.test(hex)) this.fail('invalid Unicode escape');
    this.index = hexStart + 4;
    return Number.parseInt(hex, 16);
  }

  private number(): void {
    const rest = this.source.slice(this.index);
    const match = /^(?:0|[1-9][0-9]*)/.exec(rest);
    if (!match) this.fail('numbers must use canonical nonnegative integer syntax');
    const token = match[0];
    const next = rest[token.length];
    if (next !== undefined && !/[\s,\]}]/.test(next)) this.fail('numbers must use canonical nonnegative integer syntax');
    const number = Number(token);
    if (!Number.isSafeInteger(number)) this.fail('integer exceeds JavaScript safe range');
    this.index += token.length;
  }

  private literal(value: string): boolean {
    if (!this.source.startsWith(value, this.index)) return false;
    this.index += value.length;
    return true;
  }

  private skipWhitespace(): void {
    while (/[\u0009\u000a\u000d\u0020]/.test(this.source[this.index] ?? '')) this.index++;
  }

  private take(char: string): boolean {
    if (this.source[this.index] !== char) return false;
    this.index++;
    return true;
  }

  private checkDepth(depth: number): void {
    if (depth > MAX_DEPTH) this.fail(`JSON nesting exceeds ${MAX_DEPTH}`);
  }

  private fail(message: string): never {
    throw new Error(`Invalid strict JSON at character offset ${this.index}: ${message}`);
  }
}

export function parseStrictJson(bytes: Uint8Array, maxBytes = Number.MAX_SAFE_INTEGER): unknown {
  if (bytes.byteLength > maxBytes) throw new Error(`JSON exceeds ${maxBytes} byte limit`);
  let text: string;
  try {
    text = new TextDecoder('utf-8', { fatal: true }).decode(bytes);
  } catch {
    throw new Error('JSON is not valid UTF-8');
  }
  new JsonScanner(text).scan();
  try {
    return JSON.parse(text) as unknown;
  } catch {
    throw new Error('Invalid JSON syntax');
  }
}
