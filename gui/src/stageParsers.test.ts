import { describe, expect, test } from "vitest";
import {
  parseIndentedTree,
  parseTokenTable,
  parseTypecheckDump,
} from "./stageParsers";

describe("parseIndentedTree", () => {
  test("converts compiler indented dumps into nested tree nodes", () => {
    const tree = parseIndentedTree(`Program
  Fn main -> i32
    Block
      Return
        Binary Add
`);

    expect(tree).toHaveLength(1);
    expect(tree[0].label).toBe("Program");
    expect(tree[0].children[0].label).toBe("Fn main -> i32");
    expect(tree[0].children[0].children[0].label).toBe("Block");
    expect(tree[0].children[0].children[0].children[0].label).toBe("Return");
  });

  test("extracts useful badges from ids, spans, and type suffixes", () => {
    const tree = parseIndentedTree(`HirExprId(2): i32
  Use LocalId(0) span=7..8
`);

    expect(tree[0].badges).toEqual(
      expect.arrayContaining([
        { kind: "id", text: "HirExprId(2)" },
        { kind: "type", text: "i32" },
      ]),
    );
    expect(tree[0].children[0].badges).toEqual(
      expect.arrayContaining([
        { kind: "id", text: "LocalId(0)" },
        { kind: "span", text: "7..8" },
      ]),
    );
  });
});

describe("parseTypecheckDump", () => {
  test("extracts typecheck sections as tables", () => {
    const sections = parseTypecheckDump(`TypeckResults
  DefTys
    DefId(0): fn(str, ...) -> i32
  LocalTys
    LocalId(0): &mut i32
  ExprTys
    HirExprId(1): i32
`);

    expect(sections).toEqual([
      {
        title: "DefTys",
        rows: [{ id: "DefId(0)", ty: "fn(str, ...) -> i32" }],
      },
      {
        title: "LocalTys",
        rows: [{ id: "LocalId(0)", ty: "&mut i32" }],
      },
      {
        title: "ExprTys",
        rows: [{ id: "HirExprId(1)", ty: "i32" }],
      },
    ]);
  });
});

describe("parseTokenTable", () => {
  test("extracts token rows from fixed-width token output", () => {
    const rows = parseTokenTable(`#     Kind                    Text                    Span          Line:Col
0     Keyword(Fn)             fn                      0..2          1:1
1     Id                      main                    3..7          1:4
`);

    expect(rows).toEqual([
      {
        index: "0",
        kind: "Keyword(Fn)",
        text: "fn",
        span: "0..2",
        position: "1:1",
      },
      {
        index: "1",
        kind: "Id",
        text: "main",
        span: "3..7",
        position: "1:4",
      },
    ]);
  });

  test("keeps token text containing spaces intact", () => {
    const rows = parseTokenTable(`#     Kind                    Text                    Span          Line:Col
0     Literal(String)         "answer = %d\\n"         8..22         1:9
`);

    expect(rows[0]).toEqual({
      index: "0",
      kind: "Literal(String)",
      text: '"answer = %d\\n"',
      span: "8..22",
      position: "1:9",
    });
  });
});
