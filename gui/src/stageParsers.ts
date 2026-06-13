export type StageKey =
  | "diagnostics"
  | "tokens"
  | "cst"
  | "ast"
  | "hir"
  | "typecheck"
  | "thir"
  | "ir"
  | "run";

export type BadgeKind = "id" | "span" | "type";

export type StageBadge = {
  kind: BadgeKind;
  text: string;
};

export type TreeNode = {
  id: string;
  label: string;
  depth: number;
  badges: StageBadge[];
  children: TreeNode[];
};

export type TypecheckSection = {
  title: string;
  rows: Array<{ id: string; ty: string }>;
};

export type TokenRow = {
  index: string;
  kind: string;
  text: string;
  span: string;
  position: string;
};

const ID_RE =
  /\b(?:DefId|LocalId|HirBodyId|HirExprId|HirItemId|HirStmtId|ThirBodyId|ThirExprId|ThirLocalId|ThirStmtId)\(\d+\)/g;
const SPAN_RE = /\b\d+\.\.\d+\b/g;
const TYPE_SUFFIX_RE = /^(.+?):\s*(.+)$/;

export function parseIndentedTree(text: string): TreeNode[] {
  const roots: TreeNode[] = [];
  const stack: TreeNode[] = [];

  text
    .split(/\r?\n/)
    .filter((line) => line.trim().length > 0)
    .forEach((line, index) => {
      const depth = Math.floor((line.match(/^ */)?.[0].length ?? 0) / 2);
      const content = line.trim();
      const node: TreeNode = {
        id: `${index}-${depth}-${content}`,
        label: content,
        depth,
        badges: extractBadges(content),
        children: [],
      };

      while (stack.length > depth) {
        stack.pop();
      }

      const parent = stack[stack.length - 1];
      if (parent) {
        parent.children.push(node);
      } else {
        roots.push(node);
      }

      stack[depth] = node;
    });

  return roots;
}

export function parseTypecheckDump(text: string): TypecheckSection[] {
  const sections: TypecheckSection[] = [];
  let current: TypecheckSection | null = null;

  for (const rawLine of text.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line || line === "TypeckResults") {
      continue;
    }

    if (!line.includes(":") && line !== "<empty>") {
      current = { title: line, rows: [] };
      sections.push(current);
      continue;
    }

    if (!current || line === "<empty>") {
      continue;
    }

    const [id, ...tyParts] = line.split(":");
    const ty = tyParts.join(":").trim();
    if (id && ty) {
      current.rows.push({ id: id.trim(), ty });
    }
  }

  return sections;
}

export function parseTokenTable(text: string): TokenRow[] {
  return text
    .split(/\r?\n/)
    .slice(1)
    .filter((line) => line.trim().length > 0)
    .map(parseTokenRow)
    .filter((row): row is TokenRow => row !== null);
}

function parseTokenRow(line: string): TokenRow | null {
  const row = {
    index: line.slice(0, 6).trim(),
    kind: line.slice(6, 30).trim(),
    text: line.slice(30, 54).trim(),
    span: line.slice(54, 68).trim(),
    position: line.slice(68).trim(),
  };

  if (row.index && row.kind && row.span && row.position) {
    return row;
  }

  const [index, kind, tokenText, span, position] = line.trim().split(/\s+/);
  if (!index || !kind || !tokenText || !span || !position) {
    return null;
  }

  return {
    index,
    kind,
    text: tokenText,
    span,
    position,
  };
}

function extractBadges(line: string): StageBadge[] {
  const badges: StageBadge[] = [];
  const seen = new Set<string>();

  for (const id of line.match(ID_RE) ?? []) {
    addBadge(badges, seen, "id", id);
  }

  for (const span of line.match(SPAN_RE) ?? []) {
    addBadge(badges, seen, "span", span);
  }

  const typeMatch = line.match(TYPE_SUFFIX_RE);
  if (typeMatch) {
    const ty = typeMatch[2].trim();
    if (ty && !ty.startsWith("<")) {
      addBadge(badges, seen, "type", ty);
    }
  }

  return badges;
}

function addBadge(
  badges: StageBadge[],
  seen: Set<string>,
  kind: BadgeKind,
  text: string,
) {
  const key = `${kind}:${text}`;
  if (seen.has(key)) {
    return;
  }
  seen.add(key);
  badges.push({ kind, text });
}
