import { ReactNode, useMemo, useState } from "react";
import {
  parseIndentedTree,
  parseTokenTable,
  parseTypecheckDump,
  StageKey,
  TreeNode,
} from "./stageParsers";

type StageOutputProps = {
  stage: StageKey;
  text: string;
};

const TREE_STAGES = new Set<StageKey>(["cst", "ast", "hir", "thir"]);

export function StageOutput({ stage, text }: StageOutputProps) {
  const [raw, setRaw] = useState(false);

  if (!text || text === "<not available>") {
    return <EmptyStage stage={stage} />;
  }

  if (raw) {
    return (
      <StageFrame onRawToggle={() => setRaw(false)} rawLabel="Pretty">
        <CodeOutput text={text} />
      </StageFrame>
    );
  }

  if (TREE_STAGES.has(stage)) {
    return (
      <StageFrame onRawToggle={() => setRaw(true)} rawLabel="Raw">
        <TreeStage text={text} />
      </StageFrame>
    );
  }

  if (stage === "typecheck") {
    return (
      <StageFrame onRawToggle={() => setRaw(true)} rawLabel="Raw">
        <TypecheckStage text={text} />
      </StageFrame>
    );
  }

  if (stage === "tokens") {
    return (
      <StageFrame onRawToggle={() => setRaw(true)} rawLabel="Raw">
        <TokenStage text={text} />
      </StageFrame>
    );
  }

  return <CodeOutput text={text} />;
}

function StageFrame({
  children,
  onRawToggle,
  rawLabel,
}: {
  children: ReactNode;
  onRawToggle: () => void;
  rawLabel: string;
}) {
  return (
    <div className="stage-view">
      <div className="stage-view-toolbar">
        <button type="button" onClick={onRawToggle}>
          {rawLabel}
        </button>
      </div>
      {children}
    </div>
  );
}

function TreeStage({ text }: { text: string }) {
  const roots = useMemo(() => parseIndentedTree(text), [text]);

  return (
    <div className="tree-stage">
      {roots.map((node) => (
        <TreeItem key={node.id} node={node} />
      ))}
    </div>
  );
}

function TreeItem({ node }: { node: TreeNode }) {
  const [open, setOpen] = useState(true);
  const hasChildren = node.children.length > 0;

  return (
    <div className="tree-item">
      <div className="tree-row" style={{ paddingLeft: node.depth * 14 }}>
        {hasChildren ? (
          <button
            type="button"
            className="tree-toggle"
            onClick={() => setOpen((value) => !value)}
            aria-label={open ? "Collapse node" : "Expand node"}
          >
            {open ? "▾" : "▸"}
          </button>
        ) : (
          <span className="tree-spacer" />
        )}
        <span className="tree-label">{node.label}</span>
        {node.badges.map((badge) => (
          <span key={`${badge.kind}-${badge.text}`} className={`badge ${badge.kind}`}>
            {badge.text}
          </span>
        ))}
      </div>
      {open &&
        node.children.map((child) => <TreeItem key={child.id} node={child} />)}
    </div>
  );
}

function TypecheckStage({ text }: { text: string }) {
  const sections = useMemo(() => parseTypecheckDump(text), [text]);

  return (
    <div className="typecheck-stage">
      {sections.map((section) => (
        <section key={section.title} className="type-section">
          <h3>{section.title}</h3>
          {section.rows.length === 0 ? (
            <div className="empty-inline">&lt;empty&gt;</div>
          ) : (
            <table>
              <tbody>
                {section.rows.map((row) => (
                  <tr key={`${section.title}-${row.id}`}>
                    <th>{row.id}</th>
                    <td>
                      <span className="type-pill">{row.ty}</span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </section>
      ))}
    </div>
  );
}

function TokenStage({ text }: { text: string }) {
  const rows = useMemo(() => parseTokenTable(text), [text]);

  return (
    <div className="token-stage">
      <table>
        <thead>
          <tr>
            <th>#</th>
            <th>Kind</th>
            <th>Text</th>
            <th>Span</th>
            <th>Line:Col</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => (
            <tr key={`${row.index}-${row.span}`}>
              <td>{row.index}</td>
              <td>{row.kind}</td>
              <td>{row.text}</td>
              <td>{row.span}</td>
              <td>{row.position}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function CodeOutput({ text }: { text: string }) {
  return (
    <pre className="stage-output" data-testid="code-output">
      {text}
    </pre>
  );
}

function EmptyStage({ stage }: { stage: StageKey }) {
  return (
    <div className="empty-stage">
      No {stage.toUpperCase()} output available
    </div>
  );
}
