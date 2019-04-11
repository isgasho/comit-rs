use crate::swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    rfc003::{
        self,
        alice::{self, SwapCommunication},
        bitcoin, ethereum,
        secret_source::SecretSource,
        state_machine::HtlcParams,
        Action, Actions, LedgerState,
    },
};
use bitcoin_support::{BitcoinQuantity, OutPoint};
use bitcoin_witness::PrimedInput;
use blockchain_contracts::{ethereum::rfc003::EtherHtlc, rfc003::secret::Secret};
use ethereum_support::{Bytes, EtherQuantity};

type Request = rfc003::messages::Request<Ethereum, Bitcoin, EtherQuantity, BitcoinQuantity>;
type Response = rfc003::messages::AcceptResponseBody<Ethereum, Bitcoin>;

pub fn fund_action(request: &Request, response: &Response) -> ethereum::ContractDeploy {
    HtlcParams::new_alpha_params(request, response).into()
}

pub fn refund_action(
    request: &Request,
    alpha_htlc_location: ethereum_support::Address,
) -> ethereum::SendTransaction {
    let data = Bytes::default();
    let gas_limit = EtherHtlc::tx_gas_limit();
    let network = request.alpha_ledger.network;

    ethereum::SendTransaction {
        to: alpha_htlc_location,
        data,
        gas_limit,
        amount: EtherQuantity::zero(),
        network,
    }
}

pub fn redeem_action(
    request: &Request,
    response: &Response,
    beta_htlc_location: OutPoint,
    secret_source: &dyn SecretSource,
    secret: Secret,
) -> bitcoin::SpendOutput {
    let beta_asset = request.beta_asset;
    let htlc = bitcoin::Htlc::from(HtlcParams::new_beta_params(request, response));
    let network = request.beta_ledger.network;

    bitcoin::SpendOutput {
        output: PrimedInput::new(
            beta_htlc_location,
            beta_asset,
            htlc.unlock_with_secret(secret_source.secp256k1_redeem(), &secret),
        ),
        network,
    }
}

impl Actions for alice::State<Ethereum, Bitcoin, EtherQuantity, BitcoinQuantity> {
    type ActionKind = alice::ActionKind<
        (),
        ethereum::ContractDeploy,
        bitcoin::SpendOutput,
        ethereum::SendTransaction,
    >;

    fn actions(&self) -> Vec<Action<Self::ActionKind>> {
        let (request, response) = match self.swap_communication {
            SwapCommunication::Accepted {
                ref request,
                ref response,
            } => (request, response),
            _ => return vec![],
        };
        let alpha_state = &self.alpha_ledger_state;
        let beta_state = &self.beta_ledger_state;

        use self::LedgerState::*;
        let mut actions = match alpha_state {
            NotDeployed => {
                vec![alice::ActionKind::Fund(fund_action(&request, &response)).into_action()]
            }
            Funded { htlc_location, .. } => {
                vec![
                    alice::ActionKind::Refund(refund_action(&request, *htlc_location))
                        .into_action()
                        .with_invalid_until(request.alpha_expiry),
                ]
            }
            _ => vec![],
        };

        if let Funded { htlc_location, .. } = beta_state {
            actions.push(
                alice::ActionKind::Redeem(redeem_action(
                    &request,
                    &response,
                    *htlc_location,
                    self.secret_source.as_ref(),
                    self.secret_source.secret(),
                ))
                .into_action(),
            );
        }
        actions
    }
}
