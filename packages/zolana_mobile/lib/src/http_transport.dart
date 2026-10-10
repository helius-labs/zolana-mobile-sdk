import 'dart:async';
import 'dart:typed_data';

import 'package:http/http.dart' as http;

import 'rust/third_party/zolana_mobile.dart'
    show TransportRequest, TransportResponse;

/// The wallet's transport when the application passes none: `package:http`,
/// one connection pool per wallet, closed with it.
///
/// It sends each request as it is and follows redirects. A request with
/// [TransportRequest.timeoutMs] fails when the whole of it, body included,
/// takes longer; one without fails when the response, or the next part of its
/// body, takes longer than [stall] (a proving key is several MB, so its
/// download is bounded by progress, not in all). It stops reading a body past
/// [TransportRequest.maxResponseBytes]. A body that fails after the status
/// arrived, its deadline included, is a [TransportResponseLost]: the SDK
/// gives up at its own deadline and would otherwise send a proof request
/// again.
/// `ZolanaWallet.open` checks the URLs before the wallet uses it (see
/// [isSecureUrl]).
class HttpTransport {
  HttpTransport({http.Client? client, this.stall = const Duration(seconds: 30)})
    : _client = client ?? http.Client();

  final http.Client _client;

  /// The longest wait for the response, or the next part of its body, of a
  /// request without [TransportRequest.timeoutMs].
  final Duration stall;

  Future<TransportResponse> send(TransportRequest request) async {
    final outgoing = http.Request(request.method, Uri.parse(request.url))
      ..headers.addAll(request.headers)
      ..bodyBytes = request.body;
    final timeoutMs = request.timeoutMs;
    final deadline = timeoutMs == null
        ? null
        : Duration(milliseconds: timeoutMs);
    final clock = Stopwatch()..start();
    // How long the next event may take: the stall bound, or what is left of
    // the request's deadline.
    Duration wait() {
      if (deadline == null) return stall;
      final left = deadline - clock.elapsed;
      return left.isNegative ? Duration.zero : left;
    }

    final response = await _client.send(outgoing).timeout(wait());
    final limit = request.maxResponseBytes;
    if (limit != null && (response.contentLength ?? 0) > limit) {
      unawaited(response.stream.listen(null).cancel());
      throw _TooLarge(limit);
    }
    final body = BytesBuilder(copy: false);
    final chunks = StreamIterator(response.stream);
    try {
      while (await chunks.moveNext().timeout(wait())) {
        body.add(chunks.current);
        if (limit != null && body.length > limit) throw _TooLarge(limit);
      }
    } on _TooLarge {
      unawaited(chunks.cancel());
      rethrow;
    } catch (error) {
      unawaited(chunks.cancel());
      throw TransportResponseLost(response.statusCode, error);
    }
    return TransportResponse(
      status: response.statusCode,
      headers: response.headers,
      body: body.takeBytes(),
    );
  }

  void close() => _client.close();
}

/// Thrown by a transport when the response's status arrived and its body
/// could not be read. The server got the request and may have acted on it,
/// so the wallet does not send it again: a prover would prove the spend
/// twice. The wallet fails with [cause]'s message.
class TransportResponseLost implements Exception {
  const TransportResponseLost(this.status, this.cause);

  final int status;
  final Object cause;

  @override
  String toString() => 'response body lost after status $status: $cause';
}

/// `https`, or `http` to this device.
bool isSecureUrl(String url) {
  final uri = Uri.tryParse(url);
  return uri != null &&
      (uri.scheme == 'https' ||
          (uri.scheme == 'http' &&
              const {'localhost', '127.0.0.1', '::1'}.contains(uri.host)));
}

class _TooLarge implements Exception {
  const _TooLarge(this.limit);

  final int limit;

  @override
  String toString() => 'response body exceeds $limit bytes';
}
