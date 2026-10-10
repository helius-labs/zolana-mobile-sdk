import 'package:http/http.dart' as http;
import 'package:zolana_mobile/zolana_mobile.dart';

/// The application's own networking stack as the wallet's transport: here
/// `package:http`, which follows redirects, with a timeout and a log of what
/// it sent. Pass [send] as the `transport` of `ZolanaWallet.open`.
class AppTransport {
  final _client = http.Client();

  /// Method and path of each request sent; the query, which holds the API
  /// key, is left out.
  final sent = <String>[];

  static const _timeout = Duration(seconds: 30);

  Future<TransportResponse> send(TransportRequest request) async {
    final clock = Stopwatch()..start();
    final url = Uri.parse(request.url);
    sent.add('${request.method} ${url.path}');
    // The wallet's own bound when it sets one, such as for a proof.
    final timeout = switch (request.timeoutMs) {
      final ms? => Duration(milliseconds: ms),
      null => _timeout,
    };
    final response = await _client
        .send(
          http.Request(request.method, url)
            ..headers.addAll(request.headers)
            ..bodyBytes = request.body,
        )
        .timeout(timeout);
    try {
      final body = http.ByteStream(response.stream.timeout(timeout)).toBytes();
      return TransportResponse(
        status: response.statusCode,
        headers: response.headers,
        // The wallet's bound covers the whole request, body included. A
        // proving key is several MB and has none: bound each wait instead.
        body: await (request.timeoutMs == null
            ? body
            : body.timeout(timeout - clock.elapsed)),
      );
    } catch (error) {
      // The server has the request, and a proof request must not be sent
      // twice: the wallet does not send it again after this.
      throw TransportResponseLost(response.statusCode, error);
    }
  }

  void close() => _client.close();
}
