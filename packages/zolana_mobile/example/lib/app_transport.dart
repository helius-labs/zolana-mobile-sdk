import 'package:http/http.dart' as http;
import 'package:zolana_mobile/zolana_mobile.dart';

/// The application's own networking stack as the wallet's transport: here
/// `package:http`, with a log of what it sent. Pass [send] as the `transport`
/// of `ZolanaWallet.open`.
class AppTransport {
  final _client = http.Client();

  /// Method and path of each request sent; the query, which holds the API
  /// key, is left out.
  final sent = <String>[];

  Future<TransportResponse> send(TransportRequest request) async {
    final url = Uri.parse(request.url);
    sent.add('${request.method} ${url.path}');
    final response = await _client.send(
      http.Request(request.method, url)
        ..headers.addAll(request.headers)
        ..bodyBytes = request.body,
    );
    return TransportResponse(
      status: response.statusCode,
      body: await response.stream.toBytes(),
    );
  }

  void close() => _client.close();
}
