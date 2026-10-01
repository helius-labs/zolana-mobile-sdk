import 'dart:async';

import 'package:http/http.dart';
import 'package:logging/logging.dart';

final _log = Logger('download');

const downloadAttempts = 5;

/// `GET url`, retried on what fails now and then: a reset or dropped
/// connection, a timeout, 429 and 5xx. A precompiled binary that fails to
/// download makes the plugin build from source, which needs Rust and Go, so a
/// passing blip must not end the download. Other statuses, 404 among them, are
/// returned as they are.
Future<Response> getWithRetry(
  Uri url, {
  Client? client,
  Duration Function(int attempt) delay = _backoff,
}) async {
  final http = client ?? Client();
  try {
    for (var attempt = 1;; attempt++) {
      try {
        final response = await http.get(url);
        if (!_retryable(response.statusCode) || attempt == downloadAttempts) {
          return response;
        }
        _log.warning('Failed to download $url: status ${response.statusCode}, '
            'attempt $attempt of $downloadAttempts, will retry');
      } on Exception catch (e) {
        if (attempt == downloadAttempts) rethrow;
        _log.warning('Failed to download $url: $e, '
            'attempt $attempt of $downloadAttempts, will retry');
      }
      await Future<void>.delayed(delay(attempt));
    }
  } finally {
    if (client == null) http.close();
  }
}

bool _retryable(int status) => status == 429 || status >= 500;

Duration _backoff(int attempt) => Duration(seconds: 1 << (attempt - 1));
