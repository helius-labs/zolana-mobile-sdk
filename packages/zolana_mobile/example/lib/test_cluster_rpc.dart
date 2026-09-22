import 'dart:convert';
import 'dart:io';

/// The two public Solana RPC calls the example needs on a test cluster:
/// funding the demo key and showing its public balance. Everything private
/// goes through the Zolana wallet.
class TestClusterRpc {
  TestClusterRpc(this.url);

  final String url;

  Future<BigInt> balance(String publicKey) async {
    final result = await _call('getBalance', [publicKey]);
    return BigInt.from((result as Map)['value'] as int);
  }

  /// Request [lamports] and wait until the airdrop is confirmed.
  Future<void> airdrop(String publicKey, BigInt lamports) async {
    final signature =
        await _call('requestAirdrop', [publicKey, lamports.toInt()]) as String;
    for (var attempt = 0; attempt < 60; attempt++) {
      final statuses = await _call('getSignatureStatuses', [
        [signature],
      ]);
      final status = ((statuses as Map)['value'] as List).single;
      if (status != null && status['confirmationStatus'] != 'processed') {
        return;
      }
      await Future<void>.delayed(const Duration(milliseconds: 500));
    }
    throw StateError('airdrop $signature was not confirmed');
  }

  Future<Object?> _call(String method, List<Object?> params) async {
    final client = HttpClient();
    try {
      final request = await client.postUrl(Uri.parse(url));
      request.headers.contentType = ContentType.json;
      request.write(
        jsonEncode({
          'jsonrpc': '2.0',
          'id': 1,
          'method': method,
          'params': params,
        }),
      );
      final response = await request.close();
      final body = jsonDecode(await response.transform(utf8.decoder).join());
      if (body['error'] != null) {
        throw StateError('$method failed: ${body['error']['message']}');
      }
      return body['result'];
    } finally {
      client.close();
    }
  }
}
