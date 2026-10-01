/// Two throwaway devnet accounts compiled into the example so that funding
/// survives reinstalls and one phone can pay itself privately (A → B).
///
/// Their private keys are public in this repository: anyone can spend what
/// they hold. Never send them anything but devnet SOL.
class DemoAccount {
  const DemoAccount(this.name, this.publicKey, this.seedHex);

  final String name;
  final String publicKey;

  /// 32-byte Ed25519 seed, hex.
  final String seedHex;
}

const demoAccounts = [
  DemoAccount(
    'A',
    'DzWQxW7A9N4Fo4SYiZGws6RQ2mn8rD4n92LaqTfgH93N',
    'a0a60f24c56c18101be405cc2ddb750d88961c1ee781bd17cd174fe1f5ff55dd',
  ),
  DemoAccount(
    'B',
    '8JEEsykAwsPBvtcXa5byTZbggSpC63EiXRW44khYsS8E',
    '7138835c906af341f4eec548684b5204d5b617b19573a0dd758050982601bb67',
  ),
];
