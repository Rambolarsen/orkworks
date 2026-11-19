// JSON values embedded in JavaScript also need to survive an HTML script element.
// Unicode escapes preserve the value while preventing markup from ending the script.
export function javascriptLiteral(value) {
  return JSON.stringify(value).replace(/[<>\u2028\u2029]/g, character =>
    '\\u' + character.charCodeAt(0).toString(16).padStart(4, '0'));
}
