use crate::lexer::token::Span;
use crate::typecheck::{
    error::{TypeError, TypeErrorKind},
    ty::{TyId, TyKind, TyStore, TyVarId},
};

#[derive(Debug, Clone)]
/// 类型推导上下文。
///
/// `InferCtx` 负责维护类型推导过程中产生的所有待推导类型变量，并通过并查集记录
/// 这些类型变量之间的等价关系。每个类型变量可以暂时没有确定类型，也可以在后续统一
/// 过程中绑定到一个具体的 `TyId`。
///
/// 它通常和 `TyStore` 配合使用：
///
/// 1. 类型检查遇到未知类型时，调用 `new_ty_var` 创建 `TyKind::Infer`。
/// 2. 检查表达式、语句或函数调用时，调用 `unify` 产生并求解类型等式约束。
/// 3. 推导完成后，调用 `resolve_ty` 或 `deep_resolve_ty` 得到最终类型。
///
/// 该结构只处理“类型相等”层面的局部推导，不负责名字解析、trait求解、自动借用、
/// 自动解引用或运算符重载。遇到无法统一的类型时，会返回 `TypeError`，由上层类型
/// 检查器决定如何记录诊断并继续检查。
pub struct InferCtx {
    vars: Vec<InferVar>,
    var_tys: Vec<TyId>,
}

#[derive(Debug, Clone)]
/// 单个待推导类型变量的状态。
///
/// `parent` 和 `rank` 用于并查集，表示当前类型变量所属的等价类。
/// `binding` 只保存在等价类根节点上，表示这个类型变量集合已经被确定为某个具体类型。
pub struct InferVar {
    pub parent: Option<TyVarId>,
    pub rank: u32,

    pub binding: Option<TyId>, // 绑定到TyStore中的具体类型
}

impl InferVar {
    pub fn new() -> Self {
        Self {
            parent: None,
            rank: 0,
            binding: None,
        }
    }
}

impl InferCtx {
    /// 创建一个空的类型推导上下文。
    ///
    /// 新上下文中没有任何推导变量。通常每次执行一次完整的类型检查时创建一个
    /// `InferCtx`，并在整个 `check_program` 或 `check_body` 过程中持续复用。
    pub fn new() -> Self {
        Self {
            vars: vec![],
            var_tys: vec![],
        }
    }

    /// 创建一个新的待推导类型，并返回其 `TyId`。
    ///
    /// 该函数会先分配一个新的 `TyVarId`，再向 `TyStore` 中注册对应的
    /// `TyKind::Infer(var)`。调用者通常不直接操作 `TyVarId`，而是把返回的 `TyId`
    /// 当成普通类型参与后续检查。
    ///
    /// 典型使用场景：
    ///
    /// - `let x = expr;` 中局部变量没有显式类型时，为 `x` 创建一个推导类型。
    /// - 数组、元组、函数调用等表达式需要暂时占位时，创建推导类型等待后续约束。
    /// - 双向检查中没有可用 expected type 时，为表达式创建一个未知结果类型。
    pub fn new_ty_var(&mut self, tys: &mut TyStore) -> TyId {
        let var = self.new_var_id();
        let ty = tys.intern(TyKind::Infer(var));
        self.var_tys.push(ty);
        ty
    }

    /// 分配一个新的类型变量编号。
    ///
    /// 该函数只维护 `InferCtx` 内部的并查集节点，不会向 `TyStore` 注册
    /// `TyKind::Infer`。大多数类型检查代码应该使用 `new_ty_var`，只有需要直接
    /// 处理推导变量编号的内部逻辑才应调用该函数。
    pub fn new_var_id(&mut self) -> TyVarId {
        let id = self.vars.len();
        self.vars.push(InferVar::new());
        id
    }

    /// 查找类型变量所在等价类的根节点。
    ///
    /// 这是并查集的 find 操作，会顺便进行路径压缩。返回值代表当前类型变量集合的
    /// canonical variable，读取或写入 `binding` 时应当使用这个根节点。
    pub fn find(&mut self, var: TyVarId) -> TyVarId {
        let parent = self.vars[var].parent;

        match parent {
            None => var,

            Some(parent) if parent == var => var,

            Some(parent) => {
                let root = self.find(parent);
                self.vars[var].parent = Some(root);
                root
            }
        }
    }

    /// 合并两个类型变量所在的等价类，并返回合并后的根节点。
    ///
    /// 如果两个变量都已经绑定到具体类型，则会先调用 `unify` 检查两个绑定类型是否
    /// 兼容；只有兼容时才真正合并。若其中一个变量已经绑定，另一个未绑定，合并后的
    /// 等价类继承已有绑定。若两者都未绑定，合并后的等价类仍保持未确定状态。
    ///
    /// 该函数主要服务于 `?T0 = ?T1` 这类约束。
    pub fn union(
        &mut self,
        tys: &mut TyStore,
        a: TyVarId,
        b: TyVarId,
    ) -> Result<TyVarId, TypeError> {
        let root_a_id = self.find(a);
        let root_b_id = self.find(b);

        // 已经相同，不用合并
        if root_a_id == root_b_id {
            return Ok(root_a_id);
        }

        // 处理 binding：先计算合并后的 binding，再真正合并 root。
        let binding_a = self.vars[root_a_id].binding;
        let binding_b = self.vars[root_b_id].binding;

        let merged_binding = match (binding_a, binding_b) {
            // 若两边都有binding，且不同，则要处理
            (Some(ty_a), Some(ty_b)) => {
                let ty = self.unify(tys, ty_a, ty_b)?;
                Some(ty)
            }

            // 若只有一遍有binding，则取有的那一侧
            (Some(ty), None) | (None, Some(ty)) => Some(ty),

            // 若都没有binding，则维持没有的状态
            (None, None) => None,
        };

        self.vars[root_a_id].binding = None;
        self.vars[root_b_id].binding = None;

        let merged_root = self.link_roots(root_a_id, root_b_id);
        self.vars[merged_root].binding = merged_binding;

        Ok(merged_root)
    }

    /// 根据秩选择两个并查集根节点的合并方向。
    ///
    /// 该函数只处理并查集结构本身，不处理类型绑定。调用方需要先计算并清理旧根节点
    /// 的 `binding`，再把合并后的绑定写入返回的新根节点。
    fn link_roots(&mut self, a: TyVarId, b: TyVarId) -> TyVarId {
        let rank_a = self.vars[a].rank;
        let rank_b = self.vars[b].rank;

        if rank_a < rank_b {
            self.vars[a].parent = Some(b);
            b
        } else if rank_a > rank_b {
            self.vars[b].parent = Some(a);
            a
        } else {
            self.vars[b].parent = Some(a);
            self.vars[a].rank += 1;
            a
        }
    }

    /// 将一个推导变量绑定到指定类型。
    ///
    /// 如果目标类型本身也是推导变量，则转化为两个推导变量的 `union`。如果当前变量
    /// 已经有绑定，则会把旧绑定和新类型继续 `unify`，确保同一个推导变量不会被绑定
    /// 到两个不兼容的类型。
    ///
    /// 绑定前会执行 occurs check，避免生成无限递归类型，例如把 `?T` 绑定为
    /// `&?T` 或 `(?T,)`。当前语言不支持这类递归类型，因此这种情况会返回
    /// `OccursCheckFailed`。
    pub fn bind_var(&mut self, tys: &mut TyStore, var: TyVarId, ty: TyId) -> Result<(), TypeError> {
        let root = self.find(var);

        // 目标类型是推导变量
        match tys.kind(ty).clone() {
            TyKind::Infer(var_u) => {
                self.union(tys, root, var_u)?;
                return Ok(());
            }
            _ => {}
        };

        let root = self.find(root);

        match self.vars[root].binding {
            Some(binding) => {
                let ty = self.unify(tys, binding, ty)?;
                self.vars[root].binding = Some(ty);
            }
            None => {
                if self.occurs_in(tys, root, ty) {
                    return Err(Self::error(TypeErrorKind::OccursCheckFailed {
                        var: root,
                        ty,
                    }));
                }
                self.vars[root].binding = Some(ty);
            }
        };

        Ok(())
    }

    /// 将一个类型解析到当前已知的最外层结果。
    ///
    /// 如果 `ty` 是已经绑定的推导类型，则递归返回它绑定到的类型；如果它仍未绑定，
    /// 则返回该推导变量对应的 canonical `TyId`。非推导类型会原样返回。
    ///
    /// 注意该函数只解析最外层。例如 `&?T` 中的内部 `?T` 不会被继续展开。如需把复合
    /// 类型内部的推导变量也全部解析，应使用 `deep_resolve_ty`。
    pub fn resolve_ty(&mut self, tys: &TyStore, ty: TyId) -> TyId {
        match tys.kind(ty).clone() {
            TyKind::Infer(var) => {
                let root = self.find(var);

                match self.vars[root].binding {
                    Some(binding) => self.resolve_ty(tys, binding),
                    None => self.var_tys.get(root).copied().unwrap_or(ty),
                }
            }
            _ => ty,
        }
    }

    /// 递归解析一个类型内部的所有推导变量。
    ///
    /// 该函数会先解析类型本身，如果解析后是元组、数组、引用或函数类型，则继续递归
    /// 解析其子类型，并把解析后的结构重新写入 `TyStore`。它适合在类型检查结束后
    /// 写入 `TypeckResults` 或进行 pretty dump 前使用。
    ///
    /// 如果某个推导变量仍未被绑定，结果中会保留对应的 `TyKind::Infer`，上层可以再
    /// 根据语言策略报告 `CannotInferType`。
    pub fn deep_resolve_ty(&mut self, tys: &mut TyStore, ty: TyId) -> TyId {
        let ty = self.resolve_ty(tys, ty);

        match tys.kind(ty).clone() {
            TyKind::Tuple(elems) => {
                let elems = elems
                    .into_iter()
                    .map(|elem| self.deep_resolve_ty(tys, elem))
                    .collect();
                tys.intern(TyKind::Tuple(elems))
            }
            TyKind::Array { elem, len } => {
                let elem = self.deep_resolve_ty(tys, elem);
                tys.intern(TyKind::Array { elem, len })
            }
            TyKind::Ref { mutable, inner } => {
                let inner = self.deep_resolve_ty(tys, inner);
                tys.intern(TyKind::Ref { mutable, inner })
            }
            TyKind::Fn { params, ret } => {
                let params = params
                    .into_iter()
                    .map(|param| self.deep_resolve_ty(tys, param))
                    .collect();
                let ret = self.deep_resolve_ty(tys, ret);
                tys.intern(TyKind::Fn { params, ret })
            }
            _ => ty,
        }
    }

    /// 统一两个类型，并返回统一后的类型。
    ///
    /// `expected` 表示上下文期望类型，`actual` 表示实际推导出的类型。当前实现只处理
    /// 类型相等约束：两边要么本来相同，要么可以通过绑定推导变量变成相同结构。
    ///
    /// 支持的主要规则包括：
    ///
    /// - `Error` 可以和任意类型统一，用于错误恢复。
    /// - `Never` 可以和任意类型统一，用于 `return`、`break` 等不正常返回表达式。
    /// - 推导变量可以绑定到具体类型，或和另一个推导变量合并。
    /// - 引用类型要求可变性一致，并递归统一内部类型。
    /// - 元组、数组、函数类型要求结构一致，并逐项递归统一。
    ///
    /// 如果两边结构不兼容，则返回 `MismatchedTypes`。该函数不做隐式转换、自动借用、
    /// 自动解引用、trait约束求解或运算符重载。
    pub fn unify(
        &mut self,
        tys: &mut TyStore,
        expected: TyId,
        actual: TyId,
    ) -> Result<TyId, TypeError> {
        let expected = self.resolve_ty(tys, expected);
        let actual = self.resolve_ty(tys, actual);

        if expected == actual {
            return Ok(expected);
        }

        let expected_kind = tys.kind(expected).clone();
        let actual_kind = tys.kind(actual).clone();

        match (expected_kind, actual_kind) {
            (TyKind::Error, _) | (_, TyKind::Error) => Ok(tys.error()),

            (TyKind::Never, _) => Ok(actual),
            (_, TyKind::Never) => Ok(expected),

            (TyKind::Infer(var_a), TyKind::Infer(var_b)) => {
                let root = self.union(tys, var_a, var_b)?;
                let ty = self.var_tys.get(root).copied().unwrap_or(expected);
                Ok(self.resolve_ty(tys, ty))
            }

            (TyKind::Infer(var), _) => {
                self.bind_var(tys, var, actual)?;
                Ok(actual)
            }

            (_, TyKind::Infer(var)) => {
                self.bind_var(tys, var, expected)?;
                Ok(expected)
            }

            (
                TyKind::Ref {
                    mutable: expected_mut,
                    inner: expected_inner,
                },
                TyKind::Ref {
                    mutable: actual_mut,
                    inner: actual_inner,
                },
            ) => {
                if expected_mut != actual_mut {
                    return Err(self.mismatched(expected, actual));
                }

                let inner = self.unify(tys, expected_inner, actual_inner)?;
                Ok(tys.intern(TyKind::Ref {
                    mutable: expected_mut,
                    inner,
                }))
            }

            (TyKind::Tuple(expected_elems), TyKind::Tuple(actual_elems)) => {
                if expected_elems.len() != actual_elems.len() {
                    return Err(self.mismatched(expected, actual));
                }

                let elems = expected_elems
                    .into_iter()
                    .zip(actual_elems)
                    .map(|(expected_elem, actual_elem)| self.unify(tys, expected_elem, actual_elem))
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(tys.intern(TyKind::Tuple(elems)))
            }

            (
                TyKind::Array {
                    elem: expected_elem,
                    len: expected_len,
                },
                TyKind::Array {
                    elem: actual_elem,
                    len: actual_len,
                },
            ) => {
                if expected_len != actual_len {
                    return Err(self.mismatched(expected, actual));
                }

                let elem = self.unify(tys, expected_elem, actual_elem)?;
                Ok(tys.intern(TyKind::Array {
                    elem,
                    len: expected_len,
                }))
            }

            (
                TyKind::Fn {
                    params: expected_params,
                    ret: expected_ret,
                },
                TyKind::Fn {
                    params: actual_params,
                    ret: actual_ret,
                },
            ) => {
                if expected_params.len() != actual_params.len() {
                    return Err(self.mismatched(expected, actual));
                }

                let params = expected_params
                    .into_iter()
                    .zip(actual_params)
                    .map(|(expected_param, actual_param)| {
                        self.unify(tys, expected_param, actual_param)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let ret = self.unify(tys, expected_ret, actual_ret)?;

                Ok(tys.intern(TyKind::Fn { params, ret }))
            }

            _ => Err(self.mismatched(expected, actual)),
        }
    }

    /// 判断指定推导变量是否出现在某个类型内部。
    ///
    /// 这是 occurs check 的核心逻辑，用于阻止无限类型。例如，如果允许把 `?T` 绑定为
    /// `&?T`，那么 `?T` 会展开成无限层引用，后续类型表示和代码生成都会失去有限结构。
    ///
    /// 调用该函数前后都会通过 `find` 和 `resolve_ty` 使用当前最新的等价类信息。
    fn occurs_in(&mut self, tys: &TyStore, var: TyVarId, ty: TyId) -> bool {
        let root = self.find(var);
        let ty = self.resolve_ty(tys, ty);

        match tys.kind(ty).clone() {
            TyKind::Infer(other) => self.find(other) == root,
            TyKind::Tuple(elems) => elems
                .into_iter()
                .any(|elem| self.occurs_in(tys, root, elem)),
            TyKind::Array { elem, .. } => self.occurs_in(tys, root, elem),
            TyKind::Ref { inner, .. } => self.occurs_in(tys, root, inner),
            TyKind::Fn { params, ret } => {
                params
                    .into_iter()
                    .any(|param| self.occurs_in(tys, root, param))
                    || self.occurs_in(tys, root, ret)
            }
            _ => false,
        }
    }

    /// 构造类型不匹配错误。
    ///
    /// 当前 `InferCtx` 不直接持有 HIR 节点或源码位置信息，因此这里生成的是无具体位置的
    /// 类型错误。上层类型检查器如果掌握表达式 span，可以在转换诊断前补充更准确的位置。
    fn mismatched(&self, expected: TyId, actual: TyId) -> TypeError {
        Self::error(TypeErrorKind::MismatchedTypes { expected, actual })
    }

    /// 构造推导阶段的基础类型错误。
    ///
    /// 该辅助函数只负责填充错误种类。由于类型推导上下文本身不绑定具体语法节点，
    /// 这里暂时使用空 span，具体诊断位置由调用方根据检查现场决定。
    fn error(kind: TypeErrorKind) -> TypeError {
        TypeError {
            kind,
            span: Span { start: 0, end: 0 },
        }
    }
}
