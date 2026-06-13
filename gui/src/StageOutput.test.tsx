import { render, screen } from "@testing-library/react";
import { describe, expect, test } from "vitest";
import { StageOutput } from "./StageOutput";

describe("StageOutput", () => {
  test("renders tree stages as structured nodes with raw fallback", () => {
    render(
      <StageOutput
        stage="ast"
        text={`Program
  Fn main -> i32
    Block
`}
      />,
    );

    expect(screen.getByText("Program")).toBeInTheDocument();
    expect(screen.getByText("Fn main -> i32")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Raw" })).toBeInTheDocument();
  });

  test("renders typecheck as grouped tables", () => {
    render(
      <StageOutput
        stage="typecheck"
        text={`TypeckResults
  ExprTys
    HirExprId(0): i32
`}
      />,
    );

    expect(screen.getByText("ExprTys")).toBeInTheDocument();
    expect(screen.getByText("HirExprId(0)")).toBeInTheDocument();
    expect(screen.getByText("i32")).toBeInTheDocument();
  });

  test("keeps IR in a code-oriented view", () => {
    render(
      <StageOutput
        stage="ir"
        text={`define i32 @main() {
entry:
  ret i32 0
}`}
      />,
    );

    expect(screen.getByText(/define i32 @main/)).toBeInTheDocument();
    expect(screen.getByTestId("code-output")).toBeInTheDocument();
  });

  test("renders missing stage output as an empty state", () => {
    render(<StageOutput stage="thir" text="<not available>" />);

    expect(screen.getByText("No THIR output available")).toBeInTheDocument();
  });
});
