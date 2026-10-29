import 'package:flutter/services.dart';

const _policy = String.fromEnvironment('ZOLANA_TEE_POLICY');

/// The application's reviewed policy, independent of the direct or proxy URL.
Future<String> demoProverPolicy() async => _policy.isEmpty
    ? await rootBundle.loadString('assets/policies/devnet.json')
    : _policy;
