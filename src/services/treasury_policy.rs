//! Permisos de tesorería por rol de la Fellowship.
//!
//! Separación de funciones: quien asienta un gasto no es quien lo aprueba ni lo paga.
//! Los Companions registran sus propios movimientos y los mantienen mientras siguen en
//! Planned; aprobar, pagar, presupuestar y cambiar la divisa son actos de gobierno.
//!
//! El servidor no puede leer el contenido cifrado de un movimiento, así que solo aplica
//! la regla que ve en los metadatos (un Observer nunca escribe en una ruta compartida).
//! Esta matriz es la que rige en el cliente, y es la que documenta el contrato.

use crate::models::{LedgerEntry, LedgerStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreasuryRole {
    /// Campaña propia sin Fellowship: no hay nada que restringir.
    Solo,
    Owner,
    Steward,
    Companion,
    Observer,
}

impl TreasuryRole {
    /// Un rol desconocido o ausente en una campaña compartida se trata como Observer:
    /// ante la duda, solo lectura.
    pub fn resolve(is_shared: bool, role: Option<&str>) -> Self {
        if !is_shared {
            return Self::Solo;
        }
        match role.unwrap_or("Observer") {
            "Owner" => Self::Owner,
            "Steward" => Self::Steward,
            "Companion" => Self::Companion,
            _ => Self::Observer,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Solo => "Solo Adventurer",
            Self::Owner => "Owner",
            Self::Steward => "Steward",
            Self::Companion => "Companion",
            Self::Observer => "Observer",
        }
    }

    fn governs(self) -> bool {
        matches!(self, Self::Solo | Self::Owner | Self::Steward)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreasuryAction {
    View,
    RecordEntry,
    /// Editar y borrar dependen de la autoría y de que el movimiento siga en Planned:
    /// una vez aprobado, deja de ser del Companion que lo asentó.
    EditEntry {
        mine: bool,
        status: LedgerStatus,
    },
    DeleteEntry {
        mine: bool,
        status: LedgerStatus,
    },
    ApproveEntry,
    MarkPaid,
    SetOverallBudget,
    SetCategoryBudget,
    ManageCategories,
    SwitchCurrency,
    /// Coste estimado y real de una Quest: es registro de trabajo, no de gobierno.
    SetTaskCost,
    /// Importe facturable y estado de cobro: decisión de gobierno.
    SetTaskBilling,
}

impl TreasuryAction {
    /// Contexto de edición/borrado de un movimiento concreto para la identidad activa.
    pub fn for_entry(entry: &LedgerEntry, identity: &str) -> (bool, LedgerStatus) {
        let mine = entry
            .created_by_identity
            .as_deref()
            .is_some_and(|author| author == identity);
        (mine, entry.status)
    }
}

pub fn allows(role: TreasuryRole, action: TreasuryAction) -> bool {
    use TreasuryAction::*;
    match action {
        View => true,
        RecordEntry => role != TreasuryRole::Observer,
        EditEntry { mine, status } | DeleteEntry { mine, status } => {
            role.governs() || (role == TreasuryRole::Companion && mine && status.is_open())
        }
        SetTaskCost => role != TreasuryRole::Observer,
        ApproveEntry | MarkPaid | SetOverallBudget | SetCategoryBudget | ManageCategories
        | SetTaskBilling => role.governs(),
        // La divisa es la denominación de toda la campaña: solo quien la posee la cambia.
        SwitchCurrency => matches!(role, TreasuryRole::Solo | TreasuryRole::Owner),
    }
}

/// Motivo del rechazo, ya redactado para la notificación del TUI.
pub fn denial(role: TreasuryRole, action: TreasuryAction) -> String {
    use TreasuryAction::*;
    if role == TreasuryRole::Observer {
        return "Observers may audit this Treasury, but cannot alter it.".to_string();
    }
    match action {
        ApproveEntry => "Only the Owner or a Steward may approve a Treasury entry.".to_string(),
        MarkPaid => "Only the Owner or a Steward may settle a payment.".to_string(),
        SetOverallBudget | SetCategoryBudget => {
            "Only the Owner or a Steward may set Treasury budgets.".to_string()
        }
        ManageCategories => {
            "Only the Owner or a Steward may manage Treasury categories.".to_string()
        }
        SwitchCurrency => "Only the Campaign Owner may change the Treasury currency.".to_string(),
        SetTaskBilling => {
            "Only the Owner or a Steward may set billable amounts and payment status.".to_string()
        }
        EditEntry { mine, .. } | DeleteEntry { mine, .. } => {
            if mine {
                "Once an entry leaves Planned, only the Owner or a Steward may change it."
                    .to_string()
            } else {
                "Companions may only change the Treasury entries they recorded.".to_string()
            }
        }
        View | RecordEntry | SetTaskCost => {
            format!("{} cannot perform this Treasury action.", role.label())
        }
    }
}

/// Filas de la matriz de permisos, para mostrarla en la pantalla de ayuda y en Fellowship.
pub fn capability_matrix() -> [(&'static str, [bool; 4]); 9] {
    [
        // Owner, Steward, Companion, Observer
        ("View treasury", [true, true, true, true]),
        ("Record entry", [true, true, true, false]),
        ("Edit/delete own planned", [true, true, true, false]),
        ("Edit/delete any entry", [true, true, false, false]),
        ("Approve entry", [true, true, false, false]),
        ("Settle payment", [true, true, false, false]),
        ("Set budgets", [true, true, false, false]),
        ("Manage categories", [true, true, false, false]),
        ("Switch currency", [true, false, false, false]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roles() -> [TreasuryRole; 5] {
        [
            TreasuryRole::Solo,
            TreasuryRole::Owner,
            TreasuryRole::Steward,
            TreasuryRole::Companion,
            TreasuryRole::Observer,
        ]
    }

    #[test]
    fn unknown_or_missing_role_in_a_shared_campaign_is_read_only() {
        assert_eq!(TreasuryRole::resolve(true, None), TreasuryRole::Observer);
        assert_eq!(
            TreasuryRole::resolve(true, Some("Wanderer")),
            TreasuryRole::Observer
        );
        assert_eq!(TreasuryRole::resolve(false, None), TreasuryRole::Solo);
        // Una campaña propia no se degrada por lo que diga la tabla de miembros.
        assert_eq!(
            TreasuryRole::resolve(false, Some("Observer")),
            TreasuryRole::Solo
        );
    }

    #[test]
    fn observers_can_only_look() {
        for action in [
            TreasuryAction::RecordEntry,
            TreasuryAction::ApproveEntry,
            TreasuryAction::MarkPaid,
            TreasuryAction::SetOverallBudget,
            TreasuryAction::SetCategoryBudget,
            TreasuryAction::ManageCategories,
            TreasuryAction::SwitchCurrency,
            TreasuryAction::SetTaskCost,
            TreasuryAction::SetTaskBilling,
            TreasuryAction::EditEntry {
                mine: true,
                status: LedgerStatus::Planned,
            },
            TreasuryAction::DeleteEntry {
                mine: true,
                status: LedgerStatus::Planned,
            },
        ] {
            assert!(
                !allows(TreasuryRole::Observer, action),
                "Observer must be denied {action:?}"
            );
        }
        assert!(allows(TreasuryRole::Observer, TreasuryAction::View));
    }

    #[test]
    fn companions_record_but_never_approve_or_settle() {
        assert!(allows(TreasuryRole::Companion, TreasuryAction::RecordEntry));
        assert!(allows(TreasuryRole::Companion, TreasuryAction::SetTaskCost));
        for action in [
            TreasuryAction::ApproveEntry,
            TreasuryAction::MarkPaid,
            TreasuryAction::SetOverallBudget,
            TreasuryAction::SetCategoryBudget,
            TreasuryAction::ManageCategories,
            TreasuryAction::SwitchCurrency,
            TreasuryAction::SetTaskBilling,
        ] {
            assert!(
                !allows(TreasuryRole::Companion, action),
                "Companion must be denied {action:?}"
            );
        }
    }

    #[test]
    fn companions_only_touch_their_own_open_entries() {
        let cases = [
            (true, LedgerStatus::Planned, true),
            (true, LedgerStatus::Approved, false),
            (true, LedgerStatus::Paid, false),
            (true, LedgerStatus::Cancelled, false),
            (false, LedgerStatus::Planned, false),
        ];
        for (mine, status, expected) in cases {
            assert_eq!(
                allows(
                    TreasuryRole::Companion,
                    TreasuryAction::EditEntry { mine, status }
                ),
                expected,
                "edit mine={mine} status={status:?}"
            );
            assert_eq!(
                allows(
                    TreasuryRole::Companion,
                    TreasuryAction::DeleteEntry { mine, status }
                ),
                expected,
                "delete mine={mine} status={status:?}"
            );
            // Owner y Steward no dependen de la autoría ni del estado.
            for role in [
                TreasuryRole::Owner,
                TreasuryRole::Steward,
                TreasuryRole::Solo,
            ] {
                assert!(allows(role, TreasuryAction::EditEntry { mine, status }));
                assert!(allows(role, TreasuryAction::DeleteEntry { mine, status }));
            }
        }
    }

    #[test]
    fn only_the_owner_switches_currency() {
        assert!(allows(TreasuryRole::Owner, TreasuryAction::SwitchCurrency));
        assert!(allows(TreasuryRole::Solo, TreasuryAction::SwitchCurrency));
        assert!(!allows(
            TreasuryRole::Steward,
            TreasuryAction::SwitchCurrency
        ));
    }

    #[test]
    fn every_role_and_action_has_a_denial_message() {
        for role in roles() {
            for action in [
                TreasuryAction::RecordEntry,
                TreasuryAction::ApproveEntry,
                TreasuryAction::MarkPaid,
                TreasuryAction::SetOverallBudget,
                TreasuryAction::SetCategoryBudget,
                TreasuryAction::ManageCategories,
                TreasuryAction::SwitchCurrency,
                TreasuryAction::SetTaskCost,
                TreasuryAction::SetTaskBilling,
                TreasuryAction::EditEntry {
                    mine: false,
                    status: LedgerStatus::Paid,
                },
                TreasuryAction::DeleteEntry {
                    mine: true,
                    status: LedgerStatus::Paid,
                },
            ] {
                assert!(!denial(role, action).is_empty());
            }
        }
    }

    #[test]
    fn the_matrix_matches_the_enforced_rules() {
        let columns = [
            TreasuryRole::Owner,
            TreasuryRole::Steward,
            TreasuryRole::Companion,
            TreasuryRole::Observer,
        ];
        let expectations: [(&str, TreasuryAction); 8] = [
            ("View treasury", TreasuryAction::View),
            ("Record entry", TreasuryAction::RecordEntry),
            (
                "Edit/delete own planned",
                TreasuryAction::EditEntry {
                    mine: true,
                    status: LedgerStatus::Planned,
                },
            ),
            (
                "Edit/delete any entry",
                TreasuryAction::EditEntry {
                    mine: false,
                    status: LedgerStatus::Paid,
                },
            ),
            ("Approve entry", TreasuryAction::ApproveEntry),
            ("Settle payment", TreasuryAction::MarkPaid),
            ("Set budgets", TreasuryAction::SetOverallBudget),
            ("Manage categories", TreasuryAction::ManageCategories),
        ];
        let matrix = capability_matrix();
        for (label, action) in expectations {
            let row = matrix
                .iter()
                .find(|(name, _)| *name == label)
                .unwrap_or_else(|| panic!("{label} missing from the published matrix"));
            for (index, role) in columns.into_iter().enumerate() {
                assert_eq!(
                    row.1[index],
                    allows(role, action),
                    "published matrix disagrees with enforcement for {label} / {role:?}"
                );
            }
        }
    }
}
