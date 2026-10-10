// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint, type=warning, deprecated_member_use, deprecated_member_use_from_same_package
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'zolana_mobile.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$WalletError {





@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'WalletError()';
}


}

/// @nodoc
class $WalletErrorCopyWith<$Res>  {
$WalletErrorCopyWith(WalletError _, $Res Function(WalletError) __);
}


/// Adds pattern-matching-related methods to [WalletError].
extension WalletErrorPatterns on WalletError {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( WalletError_InsufficientPrivateBalance value)?  insufficientPrivateBalance,TResult Function( WalletError_MergeRequired value)?  mergeRequired,TResult Function( WalletError_TooManyInputTrees value)?  tooManyInputTrees,TResult Function( WalletError_AmountZero value)?  amountZero,TResult Function( WalletError_NotesReserved value)?  notesReserved,TResult Function( WalletError_RecipientNotRegistered value)?  recipientNotRegistered,TResult Function( WalletError_RecipientTokenAccountMissing value)?  recipientTokenAccountMissing,TResult Function( WalletError_RegistrationConflict value)?  registrationConflict,TResult Function( WalletError_AssetNotSupported value)?  assetNotSupported,TResult Function( WalletError_MintNotConfigured value)?  mintNotConfigured,TResult Function( WalletError_InvalidMint value)?  invalidMint,TResult Function( WalletError_InvalidTokenProgram value)?  invalidTokenProgram,TResult Function( WalletError_InvalidPubkey value)?  invalidPubkey,TResult Function( WalletError_InvalidDerivationSignature value)?  invalidDerivationSignature,TResult Function( WalletError_InvalidWalletKeys value)?  invalidWalletKeys,TResult Function( WalletError_TransportFailed value)?  transportFailed,TResult Function( WalletError_SignatureInvalid value)?  signatureInvalid,TResult Function( WalletError_SignatureCountMismatch value)?  signatureCountMismatch,TResult Function( WalletError_TransactionNotConfirmed value)?  transactionNotConfirmed,TResult Function( WalletError_RemoteProverMissing value)?  remoteProverMissing,TResult Function( WalletError_ProofMalformed value)?  proofMalformed,TResult Function( WalletError_ProofInvalid value)?  proofInvalid,TResult Function( WalletError_ProofFailed value)?  proofFailed,TResult Function( WalletError_ProverBusy value)?  proverBusy,TResult Function( WalletError_ProverClosed value)?  proverClosed,TResult Function( WalletError_ProverUnavailable value)?  proverUnavailable,TResult Function( WalletError_ProverInitFailed value)?  proverInitFailed,TResult Function( WalletError_ProverLoadFailed value)?  proverLoadFailed,TResult Function( WalletError_UnsupportedCircuit value)?  unsupportedCircuit,TResult Function( WalletError_RpcUrlInsecure value)?  rpcUrlInsecure,TResult Function( WalletError_IndexerUrlInsecure value)?  indexerUrlInsecure,TResult Function( WalletError_ProvingKeyUrlInsecure value)?  provingKeyUrlInsecure,TResult Function( WalletError_ProverUrlInsecure value)?  proverUrlInsecure,TResult Function( WalletError_ProvingKeyUnknown value)?  provingKeyUnknown,TResult Function( WalletError_ProvingKeyMismatch value)?  provingKeyMismatch,TResult Function( WalletError_ProvingKeyDownloadFailed value)?  provingKeyDownloadFailed,TResult Function( WalletError_ProvingKeyCorrupt value)?  provingKeyCorrupt,TResult Function( WalletError_ProvingKeyStoreFailed value)?  provingKeyStoreFailed,TResult Function( WalletError_PoseidonInputCountInvalid value)?  poseidonInputCountInvalid,TResult Function( WalletError_PoseidonInputLengthInvalid value)?  poseidonInputLengthInvalid,TResult Function( WalletError_SignerMissing value)?  signerMissing,TResult Function( WalletError_SignerMismatch value)?  signerMismatch,TResult Function( WalletError_UnexpectedSigners value)?  unexpectedSigners,TResult Function( WalletError_WalletClosed value)?  walletClosed,TResult Function( WalletError_Client value)?  client,required TResult orElse(),}){
final _that = this;
switch (_that) {
case WalletError_InsufficientPrivateBalance() when insufficientPrivateBalance != null:
return insufficientPrivateBalance(_that);case WalletError_MergeRequired() when mergeRequired != null:
return mergeRequired(_that);case WalletError_TooManyInputTrees() when tooManyInputTrees != null:
return tooManyInputTrees(_that);case WalletError_AmountZero() when amountZero != null:
return amountZero(_that);case WalletError_NotesReserved() when notesReserved != null:
return notesReserved(_that);case WalletError_RecipientNotRegistered() when recipientNotRegistered != null:
return recipientNotRegistered(_that);case WalletError_RecipientTokenAccountMissing() when recipientTokenAccountMissing != null:
return recipientTokenAccountMissing(_that);case WalletError_RegistrationConflict() when registrationConflict != null:
return registrationConflict(_that);case WalletError_AssetNotSupported() when assetNotSupported != null:
return assetNotSupported(_that);case WalletError_MintNotConfigured() when mintNotConfigured != null:
return mintNotConfigured(_that);case WalletError_InvalidMint() when invalidMint != null:
return invalidMint(_that);case WalletError_InvalidTokenProgram() when invalidTokenProgram != null:
return invalidTokenProgram(_that);case WalletError_InvalidPubkey() when invalidPubkey != null:
return invalidPubkey(_that);case WalletError_InvalidDerivationSignature() when invalidDerivationSignature != null:
return invalidDerivationSignature(_that);case WalletError_InvalidWalletKeys() when invalidWalletKeys != null:
return invalidWalletKeys(_that);case WalletError_TransportFailed() when transportFailed != null:
return transportFailed(_that);case WalletError_SignatureInvalid() when signatureInvalid != null:
return signatureInvalid(_that);case WalletError_SignatureCountMismatch() when signatureCountMismatch != null:
return signatureCountMismatch(_that);case WalletError_TransactionNotConfirmed() when transactionNotConfirmed != null:
return transactionNotConfirmed(_that);case WalletError_RemoteProverMissing() when remoteProverMissing != null:
return remoteProverMissing(_that);case WalletError_ProofMalformed() when proofMalformed != null:
return proofMalformed(_that);case WalletError_ProofInvalid() when proofInvalid != null:
return proofInvalid(_that);case WalletError_ProofFailed() when proofFailed != null:
return proofFailed(_that);case WalletError_ProverBusy() when proverBusy != null:
return proverBusy(_that);case WalletError_ProverClosed() when proverClosed != null:
return proverClosed(_that);case WalletError_ProverUnavailable() when proverUnavailable != null:
return proverUnavailable(_that);case WalletError_ProverInitFailed() when proverInitFailed != null:
return proverInitFailed(_that);case WalletError_ProverLoadFailed() when proverLoadFailed != null:
return proverLoadFailed(_that);case WalletError_UnsupportedCircuit() when unsupportedCircuit != null:
return unsupportedCircuit(_that);case WalletError_RpcUrlInsecure() when rpcUrlInsecure != null:
return rpcUrlInsecure(_that);case WalletError_IndexerUrlInsecure() when indexerUrlInsecure != null:
return indexerUrlInsecure(_that);case WalletError_ProvingKeyUrlInsecure() when provingKeyUrlInsecure != null:
return provingKeyUrlInsecure(_that);case WalletError_ProverUrlInsecure() when proverUrlInsecure != null:
return proverUrlInsecure(_that);case WalletError_ProvingKeyUnknown() when provingKeyUnknown != null:
return provingKeyUnknown(_that);case WalletError_ProvingKeyMismatch() when provingKeyMismatch != null:
return provingKeyMismatch(_that);case WalletError_ProvingKeyDownloadFailed() when provingKeyDownloadFailed != null:
return provingKeyDownloadFailed(_that);case WalletError_ProvingKeyCorrupt() when provingKeyCorrupt != null:
return provingKeyCorrupt(_that);case WalletError_ProvingKeyStoreFailed() when provingKeyStoreFailed != null:
return provingKeyStoreFailed(_that);case WalletError_PoseidonInputCountInvalid() when poseidonInputCountInvalid != null:
return poseidonInputCountInvalid(_that);case WalletError_PoseidonInputLengthInvalid() when poseidonInputLengthInvalid != null:
return poseidonInputLengthInvalid(_that);case WalletError_SignerMissing() when signerMissing != null:
return signerMissing(_that);case WalletError_SignerMismatch() when signerMismatch != null:
return signerMismatch(_that);case WalletError_UnexpectedSigners() when unexpectedSigners != null:
return unexpectedSigners(_that);case WalletError_WalletClosed() when walletClosed != null:
return walletClosed(_that);case WalletError_Client() when client != null:
return client(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( WalletError_InsufficientPrivateBalance value)  insufficientPrivateBalance,required TResult Function( WalletError_MergeRequired value)  mergeRequired,required TResult Function( WalletError_TooManyInputTrees value)  tooManyInputTrees,required TResult Function( WalletError_AmountZero value)  amountZero,required TResult Function( WalletError_NotesReserved value)  notesReserved,required TResult Function( WalletError_RecipientNotRegistered value)  recipientNotRegistered,required TResult Function( WalletError_RecipientTokenAccountMissing value)  recipientTokenAccountMissing,required TResult Function( WalletError_RegistrationConflict value)  registrationConflict,required TResult Function( WalletError_AssetNotSupported value)  assetNotSupported,required TResult Function( WalletError_MintNotConfigured value)  mintNotConfigured,required TResult Function( WalletError_InvalidMint value)  invalidMint,required TResult Function( WalletError_InvalidTokenProgram value)  invalidTokenProgram,required TResult Function( WalletError_InvalidPubkey value)  invalidPubkey,required TResult Function( WalletError_InvalidDerivationSignature value)  invalidDerivationSignature,required TResult Function( WalletError_InvalidWalletKeys value)  invalidWalletKeys,required TResult Function( WalletError_TransportFailed value)  transportFailed,required TResult Function( WalletError_SignatureInvalid value)  signatureInvalid,required TResult Function( WalletError_SignatureCountMismatch value)  signatureCountMismatch,required TResult Function( WalletError_TransactionNotConfirmed value)  transactionNotConfirmed,required TResult Function( WalletError_RemoteProverMissing value)  remoteProverMissing,required TResult Function( WalletError_ProofMalformed value)  proofMalformed,required TResult Function( WalletError_ProofInvalid value)  proofInvalid,required TResult Function( WalletError_ProofFailed value)  proofFailed,required TResult Function( WalletError_ProverBusy value)  proverBusy,required TResult Function( WalletError_ProverClosed value)  proverClosed,required TResult Function( WalletError_ProverUnavailable value)  proverUnavailable,required TResult Function( WalletError_ProverInitFailed value)  proverInitFailed,required TResult Function( WalletError_ProverLoadFailed value)  proverLoadFailed,required TResult Function( WalletError_UnsupportedCircuit value)  unsupportedCircuit,required TResult Function( WalletError_RpcUrlInsecure value)  rpcUrlInsecure,required TResult Function( WalletError_IndexerUrlInsecure value)  indexerUrlInsecure,required TResult Function( WalletError_ProvingKeyUrlInsecure value)  provingKeyUrlInsecure,required TResult Function( WalletError_ProverUrlInsecure value)  proverUrlInsecure,required TResult Function( WalletError_ProvingKeyUnknown value)  provingKeyUnknown,required TResult Function( WalletError_ProvingKeyMismatch value)  provingKeyMismatch,required TResult Function( WalletError_ProvingKeyDownloadFailed value)  provingKeyDownloadFailed,required TResult Function( WalletError_ProvingKeyCorrupt value)  provingKeyCorrupt,required TResult Function( WalletError_ProvingKeyStoreFailed value)  provingKeyStoreFailed,required TResult Function( WalletError_PoseidonInputCountInvalid value)  poseidonInputCountInvalid,required TResult Function( WalletError_PoseidonInputLengthInvalid value)  poseidonInputLengthInvalid,required TResult Function( WalletError_SignerMissing value)  signerMissing,required TResult Function( WalletError_SignerMismatch value)  signerMismatch,required TResult Function( WalletError_UnexpectedSigners value)  unexpectedSigners,required TResult Function( WalletError_WalletClosed value)  walletClosed,required TResult Function( WalletError_Client value)  client,}){
final _that = this;
switch (_that) {
case WalletError_InsufficientPrivateBalance():
return insufficientPrivateBalance(_that);case WalletError_MergeRequired():
return mergeRequired(_that);case WalletError_TooManyInputTrees():
return tooManyInputTrees(_that);case WalletError_AmountZero():
return amountZero(_that);case WalletError_NotesReserved():
return notesReserved(_that);case WalletError_RecipientNotRegistered():
return recipientNotRegistered(_that);case WalletError_RecipientTokenAccountMissing():
return recipientTokenAccountMissing(_that);case WalletError_RegistrationConflict():
return registrationConflict(_that);case WalletError_AssetNotSupported():
return assetNotSupported(_that);case WalletError_MintNotConfigured():
return mintNotConfigured(_that);case WalletError_InvalidMint():
return invalidMint(_that);case WalletError_InvalidTokenProgram():
return invalidTokenProgram(_that);case WalletError_InvalidPubkey():
return invalidPubkey(_that);case WalletError_InvalidDerivationSignature():
return invalidDerivationSignature(_that);case WalletError_InvalidWalletKeys():
return invalidWalletKeys(_that);case WalletError_TransportFailed():
return transportFailed(_that);case WalletError_SignatureInvalid():
return signatureInvalid(_that);case WalletError_SignatureCountMismatch():
return signatureCountMismatch(_that);case WalletError_TransactionNotConfirmed():
return transactionNotConfirmed(_that);case WalletError_RemoteProverMissing():
return remoteProverMissing(_that);case WalletError_ProofMalformed():
return proofMalformed(_that);case WalletError_ProofInvalid():
return proofInvalid(_that);case WalletError_ProofFailed():
return proofFailed(_that);case WalletError_ProverBusy():
return proverBusy(_that);case WalletError_ProverClosed():
return proverClosed(_that);case WalletError_ProverUnavailable():
return proverUnavailable(_that);case WalletError_ProverInitFailed():
return proverInitFailed(_that);case WalletError_ProverLoadFailed():
return proverLoadFailed(_that);case WalletError_UnsupportedCircuit():
return unsupportedCircuit(_that);case WalletError_RpcUrlInsecure():
return rpcUrlInsecure(_that);case WalletError_IndexerUrlInsecure():
return indexerUrlInsecure(_that);case WalletError_ProvingKeyUrlInsecure():
return provingKeyUrlInsecure(_that);case WalletError_ProverUrlInsecure():
return proverUrlInsecure(_that);case WalletError_ProvingKeyUnknown():
return provingKeyUnknown(_that);case WalletError_ProvingKeyMismatch():
return provingKeyMismatch(_that);case WalletError_ProvingKeyDownloadFailed():
return provingKeyDownloadFailed(_that);case WalletError_ProvingKeyCorrupt():
return provingKeyCorrupt(_that);case WalletError_ProvingKeyStoreFailed():
return provingKeyStoreFailed(_that);case WalletError_PoseidonInputCountInvalid():
return poseidonInputCountInvalid(_that);case WalletError_PoseidonInputLengthInvalid():
return poseidonInputLengthInvalid(_that);case WalletError_SignerMissing():
return signerMissing(_that);case WalletError_SignerMismatch():
return signerMismatch(_that);case WalletError_UnexpectedSigners():
return unexpectedSigners(_that);case WalletError_WalletClosed():
return walletClosed(_that);case WalletError_Client():
return client(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( WalletError_InsufficientPrivateBalance value)?  insufficientPrivateBalance,TResult? Function( WalletError_MergeRequired value)?  mergeRequired,TResult? Function( WalletError_TooManyInputTrees value)?  tooManyInputTrees,TResult? Function( WalletError_AmountZero value)?  amountZero,TResult? Function( WalletError_NotesReserved value)?  notesReserved,TResult? Function( WalletError_RecipientNotRegistered value)?  recipientNotRegistered,TResult? Function( WalletError_RecipientTokenAccountMissing value)?  recipientTokenAccountMissing,TResult? Function( WalletError_RegistrationConflict value)?  registrationConflict,TResult? Function( WalletError_AssetNotSupported value)?  assetNotSupported,TResult? Function( WalletError_MintNotConfigured value)?  mintNotConfigured,TResult? Function( WalletError_InvalidMint value)?  invalidMint,TResult? Function( WalletError_InvalidTokenProgram value)?  invalidTokenProgram,TResult? Function( WalletError_InvalidPubkey value)?  invalidPubkey,TResult? Function( WalletError_InvalidDerivationSignature value)?  invalidDerivationSignature,TResult? Function( WalletError_InvalidWalletKeys value)?  invalidWalletKeys,TResult? Function( WalletError_TransportFailed value)?  transportFailed,TResult? Function( WalletError_SignatureInvalid value)?  signatureInvalid,TResult? Function( WalletError_SignatureCountMismatch value)?  signatureCountMismatch,TResult? Function( WalletError_TransactionNotConfirmed value)?  transactionNotConfirmed,TResult? Function( WalletError_RemoteProverMissing value)?  remoteProverMissing,TResult? Function( WalletError_ProofMalformed value)?  proofMalformed,TResult? Function( WalletError_ProofInvalid value)?  proofInvalid,TResult? Function( WalletError_ProofFailed value)?  proofFailed,TResult? Function( WalletError_ProverBusy value)?  proverBusy,TResult? Function( WalletError_ProverClosed value)?  proverClosed,TResult? Function( WalletError_ProverUnavailable value)?  proverUnavailable,TResult? Function( WalletError_ProverInitFailed value)?  proverInitFailed,TResult? Function( WalletError_ProverLoadFailed value)?  proverLoadFailed,TResult? Function( WalletError_UnsupportedCircuit value)?  unsupportedCircuit,TResult? Function( WalletError_RpcUrlInsecure value)?  rpcUrlInsecure,TResult? Function( WalletError_IndexerUrlInsecure value)?  indexerUrlInsecure,TResult? Function( WalletError_ProvingKeyUrlInsecure value)?  provingKeyUrlInsecure,TResult? Function( WalletError_ProverUrlInsecure value)?  proverUrlInsecure,TResult? Function( WalletError_ProvingKeyUnknown value)?  provingKeyUnknown,TResult? Function( WalletError_ProvingKeyMismatch value)?  provingKeyMismatch,TResult? Function( WalletError_ProvingKeyDownloadFailed value)?  provingKeyDownloadFailed,TResult? Function( WalletError_ProvingKeyCorrupt value)?  provingKeyCorrupt,TResult? Function( WalletError_ProvingKeyStoreFailed value)?  provingKeyStoreFailed,TResult? Function( WalletError_PoseidonInputCountInvalid value)?  poseidonInputCountInvalid,TResult? Function( WalletError_PoseidonInputLengthInvalid value)?  poseidonInputLengthInvalid,TResult? Function( WalletError_SignerMissing value)?  signerMissing,TResult? Function( WalletError_SignerMismatch value)?  signerMismatch,TResult? Function( WalletError_UnexpectedSigners value)?  unexpectedSigners,TResult? Function( WalletError_WalletClosed value)?  walletClosed,TResult? Function( WalletError_Client value)?  client,}){
final _that = this;
switch (_that) {
case WalletError_InsufficientPrivateBalance() when insufficientPrivateBalance != null:
return insufficientPrivateBalance(_that);case WalletError_MergeRequired() when mergeRequired != null:
return mergeRequired(_that);case WalletError_TooManyInputTrees() when tooManyInputTrees != null:
return tooManyInputTrees(_that);case WalletError_AmountZero() when amountZero != null:
return amountZero(_that);case WalletError_NotesReserved() when notesReserved != null:
return notesReserved(_that);case WalletError_RecipientNotRegistered() when recipientNotRegistered != null:
return recipientNotRegistered(_that);case WalletError_RecipientTokenAccountMissing() when recipientTokenAccountMissing != null:
return recipientTokenAccountMissing(_that);case WalletError_RegistrationConflict() when registrationConflict != null:
return registrationConflict(_that);case WalletError_AssetNotSupported() when assetNotSupported != null:
return assetNotSupported(_that);case WalletError_MintNotConfigured() when mintNotConfigured != null:
return mintNotConfigured(_that);case WalletError_InvalidMint() when invalidMint != null:
return invalidMint(_that);case WalletError_InvalidTokenProgram() when invalidTokenProgram != null:
return invalidTokenProgram(_that);case WalletError_InvalidPubkey() when invalidPubkey != null:
return invalidPubkey(_that);case WalletError_InvalidDerivationSignature() when invalidDerivationSignature != null:
return invalidDerivationSignature(_that);case WalletError_InvalidWalletKeys() when invalidWalletKeys != null:
return invalidWalletKeys(_that);case WalletError_TransportFailed() when transportFailed != null:
return transportFailed(_that);case WalletError_SignatureInvalid() when signatureInvalid != null:
return signatureInvalid(_that);case WalletError_SignatureCountMismatch() when signatureCountMismatch != null:
return signatureCountMismatch(_that);case WalletError_TransactionNotConfirmed() when transactionNotConfirmed != null:
return transactionNotConfirmed(_that);case WalletError_RemoteProverMissing() when remoteProverMissing != null:
return remoteProverMissing(_that);case WalletError_ProofMalformed() when proofMalformed != null:
return proofMalformed(_that);case WalletError_ProofInvalid() when proofInvalid != null:
return proofInvalid(_that);case WalletError_ProofFailed() when proofFailed != null:
return proofFailed(_that);case WalletError_ProverBusy() when proverBusy != null:
return proverBusy(_that);case WalletError_ProverClosed() when proverClosed != null:
return proverClosed(_that);case WalletError_ProverUnavailable() when proverUnavailable != null:
return proverUnavailable(_that);case WalletError_ProverInitFailed() when proverInitFailed != null:
return proverInitFailed(_that);case WalletError_ProverLoadFailed() when proverLoadFailed != null:
return proverLoadFailed(_that);case WalletError_UnsupportedCircuit() when unsupportedCircuit != null:
return unsupportedCircuit(_that);case WalletError_RpcUrlInsecure() when rpcUrlInsecure != null:
return rpcUrlInsecure(_that);case WalletError_IndexerUrlInsecure() when indexerUrlInsecure != null:
return indexerUrlInsecure(_that);case WalletError_ProvingKeyUrlInsecure() when provingKeyUrlInsecure != null:
return provingKeyUrlInsecure(_that);case WalletError_ProverUrlInsecure() when proverUrlInsecure != null:
return proverUrlInsecure(_that);case WalletError_ProvingKeyUnknown() when provingKeyUnknown != null:
return provingKeyUnknown(_that);case WalletError_ProvingKeyMismatch() when provingKeyMismatch != null:
return provingKeyMismatch(_that);case WalletError_ProvingKeyDownloadFailed() when provingKeyDownloadFailed != null:
return provingKeyDownloadFailed(_that);case WalletError_ProvingKeyCorrupt() when provingKeyCorrupt != null:
return provingKeyCorrupt(_that);case WalletError_ProvingKeyStoreFailed() when provingKeyStoreFailed != null:
return provingKeyStoreFailed(_that);case WalletError_PoseidonInputCountInvalid() when poseidonInputCountInvalid != null:
return poseidonInputCountInvalid(_that);case WalletError_PoseidonInputLengthInvalid() when poseidonInputLengthInvalid != null:
return poseidonInputLengthInvalid(_that);case WalletError_SignerMissing() when signerMissing != null:
return signerMissing(_that);case WalletError_SignerMismatch() when signerMismatch != null:
return signerMismatch(_that);case WalletError_UnexpectedSigners() when unexpectedSigners != null:
return unexpectedSigners(_that);case WalletError_WalletClosed() when walletClosed != null:
return walletClosed(_that);case WalletError_Client() when client != null:
return client(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( BigInt requested,  BigInt available)?  insufficientPrivateBalance,TResult Function( BigInt amount,  BigInt maxInputs)?  mergeRequired,TResult Function( BigInt trees,  BigInt maxTrees)?  tooManyInputTrees,TResult Function()?  amountZero,TResult Function( BigInt amount)?  notesReserved,TResult Function( String recipient)?  recipientNotRegistered,TResult Function( String recipient,  String mint)?  recipientTokenAccountMissing,TResult Function( String owner)?  registrationConflict,TResult Function( String mint)?  assetNotSupported,TResult Function( String mint)?  mintNotConfigured,TResult Function( String mint)?  invalidMint,TResult Function( String mint,  String tokenProgram)?  invalidTokenProgram,TResult Function( String value)?  invalidPubkey,TResult Function()?  invalidDerivationSignature,TResult Function()?  invalidWalletKeys,TResult Function( String message)?  transportFailed,TResult Function()?  signatureInvalid,TResult Function( BigInt expected,  BigInt got)?  signatureCountMismatch,TResult Function( String signature)?  transactionNotConfirmed,TResult Function()?  remoteProverMissing,TResult Function()?  proofMalformed,TResult Function()?  proofInvalid,TResult Function()?  proofFailed,TResult Function()?  proverBusy,TResult Function()?  proverClosed,TResult Function()?  proverUnavailable,TResult Function()?  proverInitFailed,TResult Function()?  proverLoadFailed,TResult Function()?  unsupportedCircuit,TResult Function( String url)?  rpcUrlInsecure,TResult Function( String url)?  indexerUrlInsecure,TResult Function( String url)?  provingKeyUrlInsecure,TResult Function( String url)?  proverUrlInsecure,TResult Function( String name)?  provingKeyUnknown,TResult Function( String name)?  provingKeyMismatch,TResult Function( String name)?  provingKeyDownloadFailed,TResult Function( String name)?  provingKeyCorrupt,TResult Function( String path)?  provingKeyStoreFailed,TResult Function( BigInt count)?  poseidonInputCountInvalid,TResult Function( BigInt index,  BigInt length)?  poseidonInputLengthInvalid,TResult Function()?  signerMissing,TResult Function( String wallet,  String signer)?  signerMismatch,TResult Function( List<String> signers)?  unexpectedSigners,TResult Function()?  walletClosed,TResult Function( String message)?  client,required TResult orElse(),}) {final _that = this;
switch (_that) {
case WalletError_InsufficientPrivateBalance() when insufficientPrivateBalance != null:
return insufficientPrivateBalance(_that.requested,_that.available);case WalletError_MergeRequired() when mergeRequired != null:
return mergeRequired(_that.amount,_that.maxInputs);case WalletError_TooManyInputTrees() when tooManyInputTrees != null:
return tooManyInputTrees(_that.trees,_that.maxTrees);case WalletError_AmountZero() when amountZero != null:
return amountZero();case WalletError_NotesReserved() when notesReserved != null:
return notesReserved(_that.amount);case WalletError_RecipientNotRegistered() when recipientNotRegistered != null:
return recipientNotRegistered(_that.recipient);case WalletError_RecipientTokenAccountMissing() when recipientTokenAccountMissing != null:
return recipientTokenAccountMissing(_that.recipient,_that.mint);case WalletError_RegistrationConflict() when registrationConflict != null:
return registrationConflict(_that.owner);case WalletError_AssetNotSupported() when assetNotSupported != null:
return assetNotSupported(_that.mint);case WalletError_MintNotConfigured() when mintNotConfigured != null:
return mintNotConfigured(_that.mint);case WalletError_InvalidMint() when invalidMint != null:
return invalidMint(_that.mint);case WalletError_InvalidTokenProgram() when invalidTokenProgram != null:
return invalidTokenProgram(_that.mint,_that.tokenProgram);case WalletError_InvalidPubkey() when invalidPubkey != null:
return invalidPubkey(_that.value);case WalletError_InvalidDerivationSignature() when invalidDerivationSignature != null:
return invalidDerivationSignature();case WalletError_InvalidWalletKeys() when invalidWalletKeys != null:
return invalidWalletKeys();case WalletError_TransportFailed() when transportFailed != null:
return transportFailed(_that.message);case WalletError_SignatureInvalid() when signatureInvalid != null:
return signatureInvalid();case WalletError_SignatureCountMismatch() when signatureCountMismatch != null:
return signatureCountMismatch(_that.expected,_that.got);case WalletError_TransactionNotConfirmed() when transactionNotConfirmed != null:
return transactionNotConfirmed(_that.signature);case WalletError_RemoteProverMissing() when remoteProverMissing != null:
return remoteProverMissing();case WalletError_ProofMalformed() when proofMalformed != null:
return proofMalformed();case WalletError_ProofInvalid() when proofInvalid != null:
return proofInvalid();case WalletError_ProofFailed() when proofFailed != null:
return proofFailed();case WalletError_ProverBusy() when proverBusy != null:
return proverBusy();case WalletError_ProverClosed() when proverClosed != null:
return proverClosed();case WalletError_ProverUnavailable() when proverUnavailable != null:
return proverUnavailable();case WalletError_ProverInitFailed() when proverInitFailed != null:
return proverInitFailed();case WalletError_ProverLoadFailed() when proverLoadFailed != null:
return proverLoadFailed();case WalletError_UnsupportedCircuit() when unsupportedCircuit != null:
return unsupportedCircuit();case WalletError_RpcUrlInsecure() when rpcUrlInsecure != null:
return rpcUrlInsecure(_that.url);case WalletError_IndexerUrlInsecure() when indexerUrlInsecure != null:
return indexerUrlInsecure(_that.url);case WalletError_ProvingKeyUrlInsecure() when provingKeyUrlInsecure != null:
return provingKeyUrlInsecure(_that.url);case WalletError_ProverUrlInsecure() when proverUrlInsecure != null:
return proverUrlInsecure(_that.url);case WalletError_ProvingKeyUnknown() when provingKeyUnknown != null:
return provingKeyUnknown(_that.name);case WalletError_ProvingKeyMismatch() when provingKeyMismatch != null:
return provingKeyMismatch(_that.name);case WalletError_ProvingKeyDownloadFailed() when provingKeyDownloadFailed != null:
return provingKeyDownloadFailed(_that.name);case WalletError_ProvingKeyCorrupt() when provingKeyCorrupt != null:
return provingKeyCorrupt(_that.name);case WalletError_ProvingKeyStoreFailed() when provingKeyStoreFailed != null:
return provingKeyStoreFailed(_that.path);case WalletError_PoseidonInputCountInvalid() when poseidonInputCountInvalid != null:
return poseidonInputCountInvalid(_that.count);case WalletError_PoseidonInputLengthInvalid() when poseidonInputLengthInvalid != null:
return poseidonInputLengthInvalid(_that.index,_that.length);case WalletError_SignerMissing() when signerMissing != null:
return signerMissing();case WalletError_SignerMismatch() when signerMismatch != null:
return signerMismatch(_that.wallet,_that.signer);case WalletError_UnexpectedSigners() when unexpectedSigners != null:
return unexpectedSigners(_that.signers);case WalletError_WalletClosed() when walletClosed != null:
return walletClosed();case WalletError_Client() when client != null:
return client(_that.message);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( BigInt requested,  BigInt available)  insufficientPrivateBalance,required TResult Function( BigInt amount,  BigInt maxInputs)  mergeRequired,required TResult Function( BigInt trees,  BigInt maxTrees)  tooManyInputTrees,required TResult Function()  amountZero,required TResult Function( BigInt amount)  notesReserved,required TResult Function( String recipient)  recipientNotRegistered,required TResult Function( String recipient,  String mint)  recipientTokenAccountMissing,required TResult Function( String owner)  registrationConflict,required TResult Function( String mint)  assetNotSupported,required TResult Function( String mint)  mintNotConfigured,required TResult Function( String mint)  invalidMint,required TResult Function( String mint,  String tokenProgram)  invalidTokenProgram,required TResult Function( String value)  invalidPubkey,required TResult Function()  invalidDerivationSignature,required TResult Function()  invalidWalletKeys,required TResult Function( String message)  transportFailed,required TResult Function()  signatureInvalid,required TResult Function( BigInt expected,  BigInt got)  signatureCountMismatch,required TResult Function( String signature)  transactionNotConfirmed,required TResult Function()  remoteProverMissing,required TResult Function()  proofMalformed,required TResult Function()  proofInvalid,required TResult Function()  proofFailed,required TResult Function()  proverBusy,required TResult Function()  proverClosed,required TResult Function()  proverUnavailable,required TResult Function()  proverInitFailed,required TResult Function()  proverLoadFailed,required TResult Function()  unsupportedCircuit,required TResult Function( String url)  rpcUrlInsecure,required TResult Function( String url)  indexerUrlInsecure,required TResult Function( String url)  provingKeyUrlInsecure,required TResult Function( String url)  proverUrlInsecure,required TResult Function( String name)  provingKeyUnknown,required TResult Function( String name)  provingKeyMismatch,required TResult Function( String name)  provingKeyDownloadFailed,required TResult Function( String name)  provingKeyCorrupt,required TResult Function( String path)  provingKeyStoreFailed,required TResult Function( BigInt count)  poseidonInputCountInvalid,required TResult Function( BigInt index,  BigInt length)  poseidonInputLengthInvalid,required TResult Function()  signerMissing,required TResult Function( String wallet,  String signer)  signerMismatch,required TResult Function( List<String> signers)  unexpectedSigners,required TResult Function()  walletClosed,required TResult Function( String message)  client,}) {final _that = this;
switch (_that) {
case WalletError_InsufficientPrivateBalance():
return insufficientPrivateBalance(_that.requested,_that.available);case WalletError_MergeRequired():
return mergeRequired(_that.amount,_that.maxInputs);case WalletError_TooManyInputTrees():
return tooManyInputTrees(_that.trees,_that.maxTrees);case WalletError_AmountZero():
return amountZero();case WalletError_NotesReserved():
return notesReserved(_that.amount);case WalletError_RecipientNotRegistered():
return recipientNotRegistered(_that.recipient);case WalletError_RecipientTokenAccountMissing():
return recipientTokenAccountMissing(_that.recipient,_that.mint);case WalletError_RegistrationConflict():
return registrationConflict(_that.owner);case WalletError_AssetNotSupported():
return assetNotSupported(_that.mint);case WalletError_MintNotConfigured():
return mintNotConfigured(_that.mint);case WalletError_InvalidMint():
return invalidMint(_that.mint);case WalletError_InvalidTokenProgram():
return invalidTokenProgram(_that.mint,_that.tokenProgram);case WalletError_InvalidPubkey():
return invalidPubkey(_that.value);case WalletError_InvalidDerivationSignature():
return invalidDerivationSignature();case WalletError_InvalidWalletKeys():
return invalidWalletKeys();case WalletError_TransportFailed():
return transportFailed(_that.message);case WalletError_SignatureInvalid():
return signatureInvalid();case WalletError_SignatureCountMismatch():
return signatureCountMismatch(_that.expected,_that.got);case WalletError_TransactionNotConfirmed():
return transactionNotConfirmed(_that.signature);case WalletError_RemoteProverMissing():
return remoteProverMissing();case WalletError_ProofMalformed():
return proofMalformed();case WalletError_ProofInvalid():
return proofInvalid();case WalletError_ProofFailed():
return proofFailed();case WalletError_ProverBusy():
return proverBusy();case WalletError_ProverClosed():
return proverClosed();case WalletError_ProverUnavailable():
return proverUnavailable();case WalletError_ProverInitFailed():
return proverInitFailed();case WalletError_ProverLoadFailed():
return proverLoadFailed();case WalletError_UnsupportedCircuit():
return unsupportedCircuit();case WalletError_RpcUrlInsecure():
return rpcUrlInsecure(_that.url);case WalletError_IndexerUrlInsecure():
return indexerUrlInsecure(_that.url);case WalletError_ProvingKeyUrlInsecure():
return provingKeyUrlInsecure(_that.url);case WalletError_ProverUrlInsecure():
return proverUrlInsecure(_that.url);case WalletError_ProvingKeyUnknown():
return provingKeyUnknown(_that.name);case WalletError_ProvingKeyMismatch():
return provingKeyMismatch(_that.name);case WalletError_ProvingKeyDownloadFailed():
return provingKeyDownloadFailed(_that.name);case WalletError_ProvingKeyCorrupt():
return provingKeyCorrupt(_that.name);case WalletError_ProvingKeyStoreFailed():
return provingKeyStoreFailed(_that.path);case WalletError_PoseidonInputCountInvalid():
return poseidonInputCountInvalid(_that.count);case WalletError_PoseidonInputLengthInvalid():
return poseidonInputLengthInvalid(_that.index,_that.length);case WalletError_SignerMissing():
return signerMissing();case WalletError_SignerMismatch():
return signerMismatch(_that.wallet,_that.signer);case WalletError_UnexpectedSigners():
return unexpectedSigners(_that.signers);case WalletError_WalletClosed():
return walletClosed();case WalletError_Client():
return client(_that.message);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( BigInt requested,  BigInt available)?  insufficientPrivateBalance,TResult? Function( BigInt amount,  BigInt maxInputs)?  mergeRequired,TResult? Function( BigInt trees,  BigInt maxTrees)?  tooManyInputTrees,TResult? Function()?  amountZero,TResult? Function( BigInt amount)?  notesReserved,TResult? Function( String recipient)?  recipientNotRegistered,TResult? Function( String recipient,  String mint)?  recipientTokenAccountMissing,TResult? Function( String owner)?  registrationConflict,TResult? Function( String mint)?  assetNotSupported,TResult? Function( String mint)?  mintNotConfigured,TResult? Function( String mint)?  invalidMint,TResult? Function( String mint,  String tokenProgram)?  invalidTokenProgram,TResult? Function( String value)?  invalidPubkey,TResult? Function()?  invalidDerivationSignature,TResult? Function()?  invalidWalletKeys,TResult? Function( String message)?  transportFailed,TResult? Function()?  signatureInvalid,TResult? Function( BigInt expected,  BigInt got)?  signatureCountMismatch,TResult? Function( String signature)?  transactionNotConfirmed,TResult? Function()?  remoteProverMissing,TResult? Function()?  proofMalformed,TResult? Function()?  proofInvalid,TResult? Function()?  proofFailed,TResult? Function()?  proverBusy,TResult? Function()?  proverClosed,TResult? Function()?  proverUnavailable,TResult? Function()?  proverInitFailed,TResult? Function()?  proverLoadFailed,TResult? Function()?  unsupportedCircuit,TResult? Function( String url)?  rpcUrlInsecure,TResult? Function( String url)?  indexerUrlInsecure,TResult? Function( String url)?  provingKeyUrlInsecure,TResult? Function( String url)?  proverUrlInsecure,TResult? Function( String name)?  provingKeyUnknown,TResult? Function( String name)?  provingKeyMismatch,TResult? Function( String name)?  provingKeyDownloadFailed,TResult? Function( String name)?  provingKeyCorrupt,TResult? Function( String path)?  provingKeyStoreFailed,TResult? Function( BigInt count)?  poseidonInputCountInvalid,TResult? Function( BigInt index,  BigInt length)?  poseidonInputLengthInvalid,TResult? Function()?  signerMissing,TResult? Function( String wallet,  String signer)?  signerMismatch,TResult? Function( List<String> signers)?  unexpectedSigners,TResult? Function()?  walletClosed,TResult? Function( String message)?  client,}) {final _that = this;
switch (_that) {
case WalletError_InsufficientPrivateBalance() when insufficientPrivateBalance != null:
return insufficientPrivateBalance(_that.requested,_that.available);case WalletError_MergeRequired() when mergeRequired != null:
return mergeRequired(_that.amount,_that.maxInputs);case WalletError_TooManyInputTrees() when tooManyInputTrees != null:
return tooManyInputTrees(_that.trees,_that.maxTrees);case WalletError_AmountZero() when amountZero != null:
return amountZero();case WalletError_NotesReserved() when notesReserved != null:
return notesReserved(_that.amount);case WalletError_RecipientNotRegistered() when recipientNotRegistered != null:
return recipientNotRegistered(_that.recipient);case WalletError_RecipientTokenAccountMissing() when recipientTokenAccountMissing != null:
return recipientTokenAccountMissing(_that.recipient,_that.mint);case WalletError_RegistrationConflict() when registrationConflict != null:
return registrationConflict(_that.owner);case WalletError_AssetNotSupported() when assetNotSupported != null:
return assetNotSupported(_that.mint);case WalletError_MintNotConfigured() when mintNotConfigured != null:
return mintNotConfigured(_that.mint);case WalletError_InvalidMint() when invalidMint != null:
return invalidMint(_that.mint);case WalletError_InvalidTokenProgram() when invalidTokenProgram != null:
return invalidTokenProgram(_that.mint,_that.tokenProgram);case WalletError_InvalidPubkey() when invalidPubkey != null:
return invalidPubkey(_that.value);case WalletError_InvalidDerivationSignature() when invalidDerivationSignature != null:
return invalidDerivationSignature();case WalletError_InvalidWalletKeys() when invalidWalletKeys != null:
return invalidWalletKeys();case WalletError_TransportFailed() when transportFailed != null:
return transportFailed(_that.message);case WalletError_SignatureInvalid() when signatureInvalid != null:
return signatureInvalid();case WalletError_SignatureCountMismatch() when signatureCountMismatch != null:
return signatureCountMismatch(_that.expected,_that.got);case WalletError_TransactionNotConfirmed() when transactionNotConfirmed != null:
return transactionNotConfirmed(_that.signature);case WalletError_RemoteProverMissing() when remoteProverMissing != null:
return remoteProverMissing();case WalletError_ProofMalformed() when proofMalformed != null:
return proofMalformed();case WalletError_ProofInvalid() when proofInvalid != null:
return proofInvalid();case WalletError_ProofFailed() when proofFailed != null:
return proofFailed();case WalletError_ProverBusy() when proverBusy != null:
return proverBusy();case WalletError_ProverClosed() when proverClosed != null:
return proverClosed();case WalletError_ProverUnavailable() when proverUnavailable != null:
return proverUnavailable();case WalletError_ProverInitFailed() when proverInitFailed != null:
return proverInitFailed();case WalletError_ProverLoadFailed() when proverLoadFailed != null:
return proverLoadFailed();case WalletError_UnsupportedCircuit() when unsupportedCircuit != null:
return unsupportedCircuit();case WalletError_RpcUrlInsecure() when rpcUrlInsecure != null:
return rpcUrlInsecure(_that.url);case WalletError_IndexerUrlInsecure() when indexerUrlInsecure != null:
return indexerUrlInsecure(_that.url);case WalletError_ProvingKeyUrlInsecure() when provingKeyUrlInsecure != null:
return provingKeyUrlInsecure(_that.url);case WalletError_ProverUrlInsecure() when proverUrlInsecure != null:
return proverUrlInsecure(_that.url);case WalletError_ProvingKeyUnknown() when provingKeyUnknown != null:
return provingKeyUnknown(_that.name);case WalletError_ProvingKeyMismatch() when provingKeyMismatch != null:
return provingKeyMismatch(_that.name);case WalletError_ProvingKeyDownloadFailed() when provingKeyDownloadFailed != null:
return provingKeyDownloadFailed(_that.name);case WalletError_ProvingKeyCorrupt() when provingKeyCorrupt != null:
return provingKeyCorrupt(_that.name);case WalletError_ProvingKeyStoreFailed() when provingKeyStoreFailed != null:
return provingKeyStoreFailed(_that.path);case WalletError_PoseidonInputCountInvalid() when poseidonInputCountInvalid != null:
return poseidonInputCountInvalid(_that.count);case WalletError_PoseidonInputLengthInvalid() when poseidonInputLengthInvalid != null:
return poseidonInputLengthInvalid(_that.index,_that.length);case WalletError_SignerMissing() when signerMissing != null:
return signerMissing();case WalletError_SignerMismatch() when signerMismatch != null:
return signerMismatch(_that.wallet,_that.signer);case WalletError_UnexpectedSigners() when unexpectedSigners != null:
return unexpectedSigners(_that.signers);case WalletError_WalletClosed() when walletClosed != null:
return walletClosed();case WalletError_Client() when client != null:
return client(_that.message);case _:
  return null;

}
}

}

/// @nodoc


class WalletError_InsufficientPrivateBalance extends WalletError {
  const WalletError_InsufficientPrivateBalance({required this.requested, required this.available}): super._();
  

 final  BigInt requested;
 final  BigInt available;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletError_InsufficientPrivateBalanceCopyWith<WalletError_InsufficientPrivateBalance> get copyWith => _$WalletError_InsufficientPrivateBalanceCopyWithImpl<WalletError_InsufficientPrivateBalance>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_InsufficientPrivateBalance&&(identical(other.requested, requested) || other.requested == requested)&&(identical(other.available, available) || other.available == available));
}


@override
int get hashCode {
    return Object.hash(runtimeType,requested,available);
}

@override
String toString() {
    return 'WalletError.insufficientPrivateBalance(requested: $requested, available: $available)';
}


}

/// @nodoc
abstract mixin class $WalletError_InsufficientPrivateBalanceCopyWith<$Res> implements $WalletErrorCopyWith<$Res> {
  factory $WalletError_InsufficientPrivateBalanceCopyWith(WalletError_InsufficientPrivateBalance value, $Res Function(WalletError_InsufficientPrivateBalance) _then) = _$WalletError_InsufficientPrivateBalanceCopyWithImpl;
@useResult
$Res call({
 BigInt requested, BigInt available
});




}
/// @nodoc
class _$WalletError_InsufficientPrivateBalanceCopyWithImpl<$Res>
    implements $WalletError_InsufficientPrivateBalanceCopyWith<$Res> {
  _$WalletError_InsufficientPrivateBalanceCopyWithImpl(this._self, this._then);

  final WalletError_InsufficientPrivateBalance _self;
  final $Res Function(WalletError_InsufficientPrivateBalance) _then;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? requested = null,Object? available = null,}) {
  return _then(WalletError_InsufficientPrivateBalance(
requested: null == requested ? _self.requested : requested // ignore: cast_nullable_to_non_nullable
as BigInt,available: null == available ? _self.available : available // ignore: cast_nullable_to_non_nullable
as BigInt,
  ));
}


}

/// @nodoc


class WalletError_MergeRequired extends WalletError {
  const WalletError_MergeRequired({required this.amount, required this.maxInputs}): super._();
  

 final  BigInt amount;
 final  BigInt maxInputs;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletError_MergeRequiredCopyWith<WalletError_MergeRequired> get copyWith => _$WalletError_MergeRequiredCopyWithImpl<WalletError_MergeRequired>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_MergeRequired&&(identical(other.amount, amount) || other.amount == amount)&&(identical(other.maxInputs, maxInputs) || other.maxInputs == maxInputs));
}


@override
int get hashCode {
    return Object.hash(runtimeType,amount,maxInputs);
}

@override
String toString() {
    return 'WalletError.mergeRequired(amount: $amount, maxInputs: $maxInputs)';
}


}

/// @nodoc
abstract mixin class $WalletError_MergeRequiredCopyWith<$Res> implements $WalletErrorCopyWith<$Res> {
  factory $WalletError_MergeRequiredCopyWith(WalletError_MergeRequired value, $Res Function(WalletError_MergeRequired) _then) = _$WalletError_MergeRequiredCopyWithImpl;
@useResult
$Res call({
 BigInt amount, BigInt maxInputs
});




}
/// @nodoc
class _$WalletError_MergeRequiredCopyWithImpl<$Res>
    implements $WalletError_MergeRequiredCopyWith<$Res> {
  _$WalletError_MergeRequiredCopyWithImpl(this._self, this._then);

  final WalletError_MergeRequired _self;
  final $Res Function(WalletError_MergeRequired) _then;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? amount = null,Object? maxInputs = null,}) {
  return _then(WalletError_MergeRequired(
amount: null == amount ? _self.amount : amount // ignore: cast_nullable_to_non_nullable
as BigInt,maxInputs: null == maxInputs ? _self.maxInputs : maxInputs // ignore: cast_nullable_to_non_nullable
as BigInt,
  ));
}


}

/// @nodoc


class WalletError_TooManyInputTrees extends WalletError {
  const WalletError_TooManyInputTrees({required this.trees, required this.maxTrees}): super._();
  

 final  BigInt trees;
 final  BigInt maxTrees;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletError_TooManyInputTreesCopyWith<WalletError_TooManyInputTrees> get copyWith => _$WalletError_TooManyInputTreesCopyWithImpl<WalletError_TooManyInputTrees>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_TooManyInputTrees&&(identical(other.trees, trees) || other.trees == trees)&&(identical(other.maxTrees, maxTrees) || other.maxTrees == maxTrees));
}


@override
int get hashCode {
    return Object.hash(runtimeType,trees,maxTrees);
}

@override
String toString() {
    return 'WalletError.tooManyInputTrees(trees: $trees, maxTrees: $maxTrees)';
}


}

/// @nodoc
abstract mixin class $WalletError_TooManyInputTreesCopyWith<$Res> implements $WalletErrorCopyWith<$Res> {
  factory $WalletError_TooManyInputTreesCopyWith(WalletError_TooManyInputTrees value, $Res Function(WalletError_TooManyInputTrees) _then) = _$WalletError_TooManyInputTreesCopyWithImpl;
@useResult
$Res call({
 BigInt trees, BigInt maxTrees
});




}
/// @nodoc
class _$WalletError_TooManyInputTreesCopyWithImpl<$Res>
    implements $WalletError_TooManyInputTreesCopyWith<$Res> {
  _$WalletError_TooManyInputTreesCopyWithImpl(this._self, this._then);

  final WalletError_TooManyInputTrees _self;
  final $Res Function(WalletError_TooManyInputTrees) _then;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? trees = null,Object? maxTrees = null,}) {
  return _then(WalletError_TooManyInputTrees(
trees: null == trees ? _self.trees : trees // ignore: cast_nullable_to_non_nullable
as BigInt,maxTrees: null == maxTrees ? _self.maxTrees : maxTrees // ignore: cast_nullable_to_non_nullable
as BigInt,
  ));
}


}

/// @nodoc


class WalletError_AmountZero extends WalletError {
  const WalletError_AmountZero(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_AmountZero);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'WalletError.amountZero()';
}


}




/// @nodoc


class WalletError_NotesReserved extends WalletError {
  const WalletError_NotesReserved({required this.amount}): super._();
  

 final  BigInt amount;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletError_NotesReservedCopyWith<WalletError_NotesReserved> get copyWith => _$WalletError_NotesReservedCopyWithImpl<WalletError_NotesReserved>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_NotesReserved&&(identical(other.amount, amount) || other.amount == amount));
}


@override
int get hashCode {
    return Object.hash(runtimeType,amount);
}

@override
String toString() {
    return 'WalletError.notesReserved(amount: $amount)';
}


}

/// @nodoc
abstract mixin class $WalletError_NotesReservedCopyWith<$Res> implements $WalletErrorCopyWith<$Res> {
  factory $WalletError_NotesReservedCopyWith(WalletError_NotesReserved value, $Res Function(WalletError_NotesReserved) _then) = _$WalletError_NotesReservedCopyWithImpl;
@useResult
$Res call({
 BigInt amount
});




}
/// @nodoc
class _$WalletError_NotesReservedCopyWithImpl<$Res>
    implements $WalletError_NotesReservedCopyWith<$Res> {
  _$WalletError_NotesReservedCopyWithImpl(this._self, this._then);

  final WalletError_NotesReserved _self;
  final $Res Function(WalletError_NotesReserved) _then;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? amount = null,}) {
  return _then(WalletError_NotesReserved(
amount: null == amount ? _self.amount : amount // ignore: cast_nullable_to_non_nullable
as BigInt,
  ));
}


}

/// @nodoc


class WalletError_RecipientNotRegistered extends WalletError {
  const WalletError_RecipientNotRegistered({required this.recipient}): super._();
  

 final  String recipient;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletError_RecipientNotRegisteredCopyWith<WalletError_RecipientNotRegistered> get copyWith => _$WalletError_RecipientNotRegisteredCopyWithImpl<WalletError_RecipientNotRegistered>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_RecipientNotRegistered&&(identical(other.recipient, recipient) || other.recipient == recipient));
}


@override
int get hashCode {
    return Object.hash(runtimeType,recipient);
}

@override
String toString() {
    return 'WalletError.recipientNotRegistered(recipient: $recipient)';
}


}

/// @nodoc
abstract mixin class $WalletError_RecipientNotRegisteredCopyWith<$Res> implements $WalletErrorCopyWith<$Res> {
  factory $WalletError_RecipientNotRegisteredCopyWith(WalletError_RecipientNotRegistered value, $Res Function(WalletError_RecipientNotRegistered) _then) = _$WalletError_RecipientNotRegisteredCopyWithImpl;
@useResult
$Res call({
 String recipient
});




}
/// @nodoc
class _$WalletError_RecipientNotRegisteredCopyWithImpl<$Res>
    implements $WalletError_RecipientNotRegisteredCopyWith<$Res> {
  _$WalletError_RecipientNotRegisteredCopyWithImpl(this._self, this._then);

  final WalletError_RecipientNotRegistered _self;
  final $Res Function(WalletError_RecipientNotRegistered) _then;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? recipient = null,}) {
  return _then(WalletError_RecipientNotRegistered(
recipient: null == recipient ? _self.recipient : recipient // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class WalletError_RecipientTokenAccountMissing extends WalletError {
  const WalletError_RecipientTokenAccountMissing({required this.recipient, required this.mint}): super._();
  

 final  String recipient;
 final  String mint;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletError_RecipientTokenAccountMissingCopyWith<WalletError_RecipientTokenAccountMissing> get copyWith => _$WalletError_RecipientTokenAccountMissingCopyWithImpl<WalletError_RecipientTokenAccountMissing>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_RecipientTokenAccountMissing&&(identical(other.recipient, recipient) || other.recipient == recipient)&&(identical(other.mint, mint) || other.mint == mint));
}


@override
int get hashCode {
    return Object.hash(runtimeType,recipient,mint);
}

@override
String toString() {
    return 'WalletError.recipientTokenAccountMissing(recipient: $recipient, mint: $mint)';
}


}

/// @nodoc
abstract mixin class $WalletError_RecipientTokenAccountMissingCopyWith<$Res> implements $WalletErrorCopyWith<$Res> {
  factory $WalletError_RecipientTokenAccountMissingCopyWith(WalletError_RecipientTokenAccountMissing value, $Res Function(WalletError_RecipientTokenAccountMissing) _then) = _$WalletError_RecipientTokenAccountMissingCopyWithImpl;
@useResult
$Res call({
 String recipient, String mint
});




}
/// @nodoc
class _$WalletError_RecipientTokenAccountMissingCopyWithImpl<$Res>
    implements $WalletError_RecipientTokenAccountMissingCopyWith<$Res> {
  _$WalletError_RecipientTokenAccountMissingCopyWithImpl(this._self, this._then);

  final WalletError_RecipientTokenAccountMissing _self;
  final $Res Function(WalletError_RecipientTokenAccountMissing) _then;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? recipient = null,Object? mint = null,}) {
  return _then(WalletError_RecipientTokenAccountMissing(
recipient: null == recipient ? _self.recipient : recipient // ignore: cast_nullable_to_non_nullable
as String,mint: null == mint ? _self.mint : mint // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class WalletError_RegistrationConflict extends WalletError {
  const WalletError_RegistrationConflict({required this.owner}): super._();
  

 final  String owner;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletError_RegistrationConflictCopyWith<WalletError_RegistrationConflict> get copyWith => _$WalletError_RegistrationConflictCopyWithImpl<WalletError_RegistrationConflict>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_RegistrationConflict&&(identical(other.owner, owner) || other.owner == owner));
}


@override
int get hashCode {
    return Object.hash(runtimeType,owner);
}

@override
String toString() {
    return 'WalletError.registrationConflict(owner: $owner)';
}


}

/// @nodoc
abstract mixin class $WalletError_RegistrationConflictCopyWith<$Res> implements $WalletErrorCopyWith<$Res> {
  factory $WalletError_RegistrationConflictCopyWith(WalletError_RegistrationConflict value, $Res Function(WalletError_RegistrationConflict) _then) = _$WalletError_RegistrationConflictCopyWithImpl;
@useResult
$Res call({
 String owner
});




}
/// @nodoc
class _$WalletError_RegistrationConflictCopyWithImpl<$Res>
    implements $WalletError_RegistrationConflictCopyWith<$Res> {
  _$WalletError_RegistrationConflictCopyWithImpl(this._self, this._then);

  final WalletError_RegistrationConflict _self;
  final $Res Function(WalletError_RegistrationConflict) _then;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? owner = null,}) {
  return _then(WalletError_RegistrationConflict(
owner: null == owner ? _self.owner : owner // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class WalletError_AssetNotSupported extends WalletError {
  const WalletError_AssetNotSupported({required this.mint}): super._();
  

 final  String mint;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletError_AssetNotSupportedCopyWith<WalletError_AssetNotSupported> get copyWith => _$WalletError_AssetNotSupportedCopyWithImpl<WalletError_AssetNotSupported>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_AssetNotSupported&&(identical(other.mint, mint) || other.mint == mint));
}


@override
int get hashCode {
    return Object.hash(runtimeType,mint);
}

@override
String toString() {
    return 'WalletError.assetNotSupported(mint: $mint)';
}


}

/// @nodoc
abstract mixin class $WalletError_AssetNotSupportedCopyWith<$Res> implements $WalletErrorCopyWith<$Res> {
  factory $WalletError_AssetNotSupportedCopyWith(WalletError_AssetNotSupported value, $Res Function(WalletError_AssetNotSupported) _then) = _$WalletError_AssetNotSupportedCopyWithImpl;
@useResult
$Res call({
 String mint
});




}
/// @nodoc
class _$WalletError_AssetNotSupportedCopyWithImpl<$Res>
    implements $WalletError_AssetNotSupportedCopyWith<$Res> {
  _$WalletError_AssetNotSupportedCopyWithImpl(this._self, this._then);

  final WalletError_AssetNotSupported _self;
  final $Res Function(WalletError_AssetNotSupported) _then;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? mint = null,}) {
  return _then(WalletError_AssetNotSupported(
mint: null == mint ? _self.mint : mint // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class WalletError_MintNotConfigured extends WalletError {
  const WalletError_MintNotConfigured({required this.mint}): super._();
  

 final  String mint;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletError_MintNotConfiguredCopyWith<WalletError_MintNotConfigured> get copyWith => _$WalletError_MintNotConfiguredCopyWithImpl<WalletError_MintNotConfigured>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_MintNotConfigured&&(identical(other.mint, mint) || other.mint == mint));
}


@override
int get hashCode {
    return Object.hash(runtimeType,mint);
}

@override
String toString() {
    return 'WalletError.mintNotConfigured(mint: $mint)';
}


}

/// @nodoc
abstract mixin class $WalletError_MintNotConfiguredCopyWith<$Res> implements $WalletErrorCopyWith<$Res> {
  factory $WalletError_MintNotConfiguredCopyWith(WalletError_MintNotConfigured value, $Res Function(WalletError_MintNotConfigured) _then) = _$WalletError_MintNotConfiguredCopyWithImpl;
@useResult
$Res call({
 String mint
});




}
/// @nodoc
class _$WalletError_MintNotConfiguredCopyWithImpl<$Res>
    implements $WalletError_MintNotConfiguredCopyWith<$Res> {
  _$WalletError_MintNotConfiguredCopyWithImpl(this._self, this._then);

  final WalletError_MintNotConfigured _self;
  final $Res Function(WalletError_MintNotConfigured) _then;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? mint = null,}) {
  return _then(WalletError_MintNotConfigured(
mint: null == mint ? _self.mint : mint // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class WalletError_InvalidMint extends WalletError {
  const WalletError_InvalidMint({required this.mint}): super._();
  

 final  String mint;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletError_InvalidMintCopyWith<WalletError_InvalidMint> get copyWith => _$WalletError_InvalidMintCopyWithImpl<WalletError_InvalidMint>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_InvalidMint&&(identical(other.mint, mint) || other.mint == mint));
}


@override
int get hashCode {
    return Object.hash(runtimeType,mint);
}

@override
String toString() {
    return 'WalletError.invalidMint(mint: $mint)';
}


}

/// @nodoc
abstract mixin class $WalletError_InvalidMintCopyWith<$Res> implements $WalletErrorCopyWith<$Res> {
  factory $WalletError_InvalidMintCopyWith(WalletError_InvalidMint value, $Res Function(WalletError_InvalidMint) _then) = _$WalletError_InvalidMintCopyWithImpl;
@useResult
$Res call({
 String mint
});




}
/// @nodoc
class _$WalletError_InvalidMintCopyWithImpl<$Res>
    implements $WalletError_InvalidMintCopyWith<$Res> {
  _$WalletError_InvalidMintCopyWithImpl(this._self, this._then);

  final WalletError_InvalidMint _self;
  final $Res Function(WalletError_InvalidMint) _then;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? mint = null,}) {
  return _then(WalletError_InvalidMint(
mint: null == mint ? _self.mint : mint // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class WalletError_InvalidTokenProgram extends WalletError {
  const WalletError_InvalidTokenProgram({required this.mint, required this.tokenProgram}): super._();
  

 final  String mint;
 final  String tokenProgram;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletError_InvalidTokenProgramCopyWith<WalletError_InvalidTokenProgram> get copyWith => _$WalletError_InvalidTokenProgramCopyWithImpl<WalletError_InvalidTokenProgram>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_InvalidTokenProgram&&(identical(other.mint, mint) || other.mint == mint)&&(identical(other.tokenProgram, tokenProgram) || other.tokenProgram == tokenProgram));
}


@override
int get hashCode {
    return Object.hash(runtimeType,mint,tokenProgram);
}

@override
String toString() {
    return 'WalletError.invalidTokenProgram(mint: $mint, tokenProgram: $tokenProgram)';
}


}

/// @nodoc
abstract mixin class $WalletError_InvalidTokenProgramCopyWith<$Res> implements $WalletErrorCopyWith<$Res> {
  factory $WalletError_InvalidTokenProgramCopyWith(WalletError_InvalidTokenProgram value, $Res Function(WalletError_InvalidTokenProgram) _then) = _$WalletError_InvalidTokenProgramCopyWithImpl;
@useResult
$Res call({
 String mint, String tokenProgram
});




}
/// @nodoc
class _$WalletError_InvalidTokenProgramCopyWithImpl<$Res>
    implements $WalletError_InvalidTokenProgramCopyWith<$Res> {
  _$WalletError_InvalidTokenProgramCopyWithImpl(this._self, this._then);

  final WalletError_InvalidTokenProgram _self;
  final $Res Function(WalletError_InvalidTokenProgram) _then;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? mint = null,Object? tokenProgram = null,}) {
  return _then(WalletError_InvalidTokenProgram(
mint: null == mint ? _self.mint : mint // ignore: cast_nullable_to_non_nullable
as String,tokenProgram: null == tokenProgram ? _self.tokenProgram : tokenProgram // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class WalletError_InvalidPubkey extends WalletError {
  const WalletError_InvalidPubkey({required this.value}): super._();
  

 final  String value;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletError_InvalidPubkeyCopyWith<WalletError_InvalidPubkey> get copyWith => _$WalletError_InvalidPubkeyCopyWithImpl<WalletError_InvalidPubkey>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_InvalidPubkey&&(identical(other.value, value) || other.value == value));
}


@override
int get hashCode {
    return Object.hash(runtimeType,value);
}

@override
String toString() {
    return 'WalletError.invalidPubkey(value: $value)';
}


}

/// @nodoc
abstract mixin class $WalletError_InvalidPubkeyCopyWith<$Res> implements $WalletErrorCopyWith<$Res> {
  factory $WalletError_InvalidPubkeyCopyWith(WalletError_InvalidPubkey value, $Res Function(WalletError_InvalidPubkey) _then) = _$WalletError_InvalidPubkeyCopyWithImpl;
@useResult
$Res call({
 String value
});




}
/// @nodoc
class _$WalletError_InvalidPubkeyCopyWithImpl<$Res>
    implements $WalletError_InvalidPubkeyCopyWith<$Res> {
  _$WalletError_InvalidPubkeyCopyWithImpl(this._self, this._then);

  final WalletError_InvalidPubkey _self;
  final $Res Function(WalletError_InvalidPubkey) _then;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? value = null,}) {
  return _then(WalletError_InvalidPubkey(
value: null == value ? _self.value : value // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class WalletError_InvalidDerivationSignature extends WalletError {
  const WalletError_InvalidDerivationSignature(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_InvalidDerivationSignature);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'WalletError.invalidDerivationSignature()';
}


}




/// @nodoc


class WalletError_InvalidWalletKeys extends WalletError {
  const WalletError_InvalidWalletKeys(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_InvalidWalletKeys);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'WalletError.invalidWalletKeys()';
}


}




/// @nodoc


class WalletError_TransportFailed extends WalletError {
  const WalletError_TransportFailed({required this.message}): super._();
  

 final  String message;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletError_TransportFailedCopyWith<WalletError_TransportFailed> get copyWith => _$WalletError_TransportFailedCopyWithImpl<WalletError_TransportFailed>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_TransportFailed&&(identical(other.message, message) || other.message == message));
}


@override
int get hashCode {
    return Object.hash(runtimeType,message);
}

@override
String toString() {
    return 'WalletError.transportFailed(message: $message)';
}


}

/// @nodoc
abstract mixin class $WalletError_TransportFailedCopyWith<$Res> implements $WalletErrorCopyWith<$Res> {
  factory $WalletError_TransportFailedCopyWith(WalletError_TransportFailed value, $Res Function(WalletError_TransportFailed) _then) = _$WalletError_TransportFailedCopyWithImpl;
@useResult
$Res call({
 String message
});




}
/// @nodoc
class _$WalletError_TransportFailedCopyWithImpl<$Res>
    implements $WalletError_TransportFailedCopyWith<$Res> {
  _$WalletError_TransportFailedCopyWithImpl(this._self, this._then);

  final WalletError_TransportFailed _self;
  final $Res Function(WalletError_TransportFailed) _then;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? message = null,}) {
  return _then(WalletError_TransportFailed(
message: null == message ? _self.message : message // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class WalletError_SignatureInvalid extends WalletError {
  const WalletError_SignatureInvalid(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_SignatureInvalid);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'WalletError.signatureInvalid()';
}


}




/// @nodoc


class WalletError_SignatureCountMismatch extends WalletError {
  const WalletError_SignatureCountMismatch({required this.expected, required this.got}): super._();
  

 final  BigInt expected;
 final  BigInt got;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletError_SignatureCountMismatchCopyWith<WalletError_SignatureCountMismatch> get copyWith => _$WalletError_SignatureCountMismatchCopyWithImpl<WalletError_SignatureCountMismatch>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_SignatureCountMismatch&&(identical(other.expected, expected) || other.expected == expected)&&(identical(other.got, got) || other.got == got));
}


@override
int get hashCode {
    return Object.hash(runtimeType,expected,got);
}

@override
String toString() {
    return 'WalletError.signatureCountMismatch(expected: $expected, got: $got)';
}


}

/// @nodoc
abstract mixin class $WalletError_SignatureCountMismatchCopyWith<$Res> implements $WalletErrorCopyWith<$Res> {
  factory $WalletError_SignatureCountMismatchCopyWith(WalletError_SignatureCountMismatch value, $Res Function(WalletError_SignatureCountMismatch) _then) = _$WalletError_SignatureCountMismatchCopyWithImpl;
@useResult
$Res call({
 BigInt expected, BigInt got
});




}
/// @nodoc
class _$WalletError_SignatureCountMismatchCopyWithImpl<$Res>
    implements $WalletError_SignatureCountMismatchCopyWith<$Res> {
  _$WalletError_SignatureCountMismatchCopyWithImpl(this._self, this._then);

  final WalletError_SignatureCountMismatch _self;
  final $Res Function(WalletError_SignatureCountMismatch) _then;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? expected = null,Object? got = null,}) {
  return _then(WalletError_SignatureCountMismatch(
expected: null == expected ? _self.expected : expected // ignore: cast_nullable_to_non_nullable
as BigInt,got: null == got ? _self.got : got // ignore: cast_nullable_to_non_nullable
as BigInt,
  ));
}


}

/// @nodoc


class WalletError_TransactionNotConfirmed extends WalletError {
  const WalletError_TransactionNotConfirmed({required this.signature}): super._();
  

 final  String signature;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletError_TransactionNotConfirmedCopyWith<WalletError_TransactionNotConfirmed> get copyWith => _$WalletError_TransactionNotConfirmedCopyWithImpl<WalletError_TransactionNotConfirmed>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_TransactionNotConfirmed&&(identical(other.signature, signature) || other.signature == signature));
}


@override
int get hashCode {
    return Object.hash(runtimeType,signature);
}

@override
String toString() {
    return 'WalletError.transactionNotConfirmed(signature: $signature)';
}


}

/// @nodoc
abstract mixin class $WalletError_TransactionNotConfirmedCopyWith<$Res> implements $WalletErrorCopyWith<$Res> {
  factory $WalletError_TransactionNotConfirmedCopyWith(WalletError_TransactionNotConfirmed value, $Res Function(WalletError_TransactionNotConfirmed) _then) = _$WalletError_TransactionNotConfirmedCopyWithImpl;
@useResult
$Res call({
 String signature
});




}
/// @nodoc
class _$WalletError_TransactionNotConfirmedCopyWithImpl<$Res>
    implements $WalletError_TransactionNotConfirmedCopyWith<$Res> {
  _$WalletError_TransactionNotConfirmedCopyWithImpl(this._self, this._then);

  final WalletError_TransactionNotConfirmed _self;
  final $Res Function(WalletError_TransactionNotConfirmed) _then;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? signature = null,}) {
  return _then(WalletError_TransactionNotConfirmed(
signature: null == signature ? _self.signature : signature // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class WalletError_RemoteProverMissing extends WalletError {
  const WalletError_RemoteProverMissing(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_RemoteProverMissing);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'WalletError.remoteProverMissing()';
}


}




/// @nodoc


class WalletError_ProofMalformed extends WalletError {
  const WalletError_ProofMalformed(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_ProofMalformed);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'WalletError.proofMalformed()';
}


}




/// @nodoc


class WalletError_ProofInvalid extends WalletError {
  const WalletError_ProofInvalid(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_ProofInvalid);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'WalletError.proofInvalid()';
}


}




/// @nodoc


class WalletError_ProofFailed extends WalletError {
  const WalletError_ProofFailed(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_ProofFailed);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'WalletError.proofFailed()';
}


}




/// @nodoc


class WalletError_ProverBusy extends WalletError {
  const WalletError_ProverBusy(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_ProverBusy);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'WalletError.proverBusy()';
}


}




/// @nodoc


class WalletError_ProverClosed extends WalletError {
  const WalletError_ProverClosed(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_ProverClosed);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'WalletError.proverClosed()';
}


}




/// @nodoc


class WalletError_ProverUnavailable extends WalletError {
  const WalletError_ProverUnavailable(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_ProverUnavailable);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'WalletError.proverUnavailable()';
}


}




/// @nodoc


class WalletError_ProverInitFailed extends WalletError {
  const WalletError_ProverInitFailed(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_ProverInitFailed);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'WalletError.proverInitFailed()';
}


}




/// @nodoc


class WalletError_ProverLoadFailed extends WalletError {
  const WalletError_ProverLoadFailed(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_ProverLoadFailed);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'WalletError.proverLoadFailed()';
}


}




/// @nodoc


class WalletError_UnsupportedCircuit extends WalletError {
  const WalletError_UnsupportedCircuit(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_UnsupportedCircuit);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'WalletError.unsupportedCircuit()';
}


}




/// @nodoc


class WalletError_RpcUrlInsecure extends WalletError {
  const WalletError_RpcUrlInsecure({required this.url}): super._();
  

 final  String url;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletError_RpcUrlInsecureCopyWith<WalletError_RpcUrlInsecure> get copyWith => _$WalletError_RpcUrlInsecureCopyWithImpl<WalletError_RpcUrlInsecure>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_RpcUrlInsecure&&(identical(other.url, url) || other.url == url));
}


@override
int get hashCode {
    return Object.hash(runtimeType,url);
}

@override
String toString() {
    return 'WalletError.rpcUrlInsecure(url: $url)';
}


}

/// @nodoc
abstract mixin class $WalletError_RpcUrlInsecureCopyWith<$Res> implements $WalletErrorCopyWith<$Res> {
  factory $WalletError_RpcUrlInsecureCopyWith(WalletError_RpcUrlInsecure value, $Res Function(WalletError_RpcUrlInsecure) _then) = _$WalletError_RpcUrlInsecureCopyWithImpl;
@useResult
$Res call({
 String url
});




}
/// @nodoc
class _$WalletError_RpcUrlInsecureCopyWithImpl<$Res>
    implements $WalletError_RpcUrlInsecureCopyWith<$Res> {
  _$WalletError_RpcUrlInsecureCopyWithImpl(this._self, this._then);

  final WalletError_RpcUrlInsecure _self;
  final $Res Function(WalletError_RpcUrlInsecure) _then;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? url = null,}) {
  return _then(WalletError_RpcUrlInsecure(
url: null == url ? _self.url : url // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class WalletError_IndexerUrlInsecure extends WalletError {
  const WalletError_IndexerUrlInsecure({required this.url}): super._();
  

 final  String url;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletError_IndexerUrlInsecureCopyWith<WalletError_IndexerUrlInsecure> get copyWith => _$WalletError_IndexerUrlInsecureCopyWithImpl<WalletError_IndexerUrlInsecure>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_IndexerUrlInsecure&&(identical(other.url, url) || other.url == url));
}


@override
int get hashCode {
    return Object.hash(runtimeType,url);
}

@override
String toString() {
    return 'WalletError.indexerUrlInsecure(url: $url)';
}


}

/// @nodoc
abstract mixin class $WalletError_IndexerUrlInsecureCopyWith<$Res> implements $WalletErrorCopyWith<$Res> {
  factory $WalletError_IndexerUrlInsecureCopyWith(WalletError_IndexerUrlInsecure value, $Res Function(WalletError_IndexerUrlInsecure) _then) = _$WalletError_IndexerUrlInsecureCopyWithImpl;
@useResult
$Res call({
 String url
});




}
/// @nodoc
class _$WalletError_IndexerUrlInsecureCopyWithImpl<$Res>
    implements $WalletError_IndexerUrlInsecureCopyWith<$Res> {
  _$WalletError_IndexerUrlInsecureCopyWithImpl(this._self, this._then);

  final WalletError_IndexerUrlInsecure _self;
  final $Res Function(WalletError_IndexerUrlInsecure) _then;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? url = null,}) {
  return _then(WalletError_IndexerUrlInsecure(
url: null == url ? _self.url : url // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class WalletError_ProvingKeyUrlInsecure extends WalletError {
  const WalletError_ProvingKeyUrlInsecure({required this.url}): super._();
  

 final  String url;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletError_ProvingKeyUrlInsecureCopyWith<WalletError_ProvingKeyUrlInsecure> get copyWith => _$WalletError_ProvingKeyUrlInsecureCopyWithImpl<WalletError_ProvingKeyUrlInsecure>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_ProvingKeyUrlInsecure&&(identical(other.url, url) || other.url == url));
}


@override
int get hashCode {
    return Object.hash(runtimeType,url);
}

@override
String toString() {
    return 'WalletError.provingKeyUrlInsecure(url: $url)';
}


}

/// @nodoc
abstract mixin class $WalletError_ProvingKeyUrlInsecureCopyWith<$Res> implements $WalletErrorCopyWith<$Res> {
  factory $WalletError_ProvingKeyUrlInsecureCopyWith(WalletError_ProvingKeyUrlInsecure value, $Res Function(WalletError_ProvingKeyUrlInsecure) _then) = _$WalletError_ProvingKeyUrlInsecureCopyWithImpl;
@useResult
$Res call({
 String url
});




}
/// @nodoc
class _$WalletError_ProvingKeyUrlInsecureCopyWithImpl<$Res>
    implements $WalletError_ProvingKeyUrlInsecureCopyWith<$Res> {
  _$WalletError_ProvingKeyUrlInsecureCopyWithImpl(this._self, this._then);

  final WalletError_ProvingKeyUrlInsecure _self;
  final $Res Function(WalletError_ProvingKeyUrlInsecure) _then;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? url = null,}) {
  return _then(WalletError_ProvingKeyUrlInsecure(
url: null == url ? _self.url : url // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class WalletError_ProverUrlInsecure extends WalletError {
  const WalletError_ProverUrlInsecure({required this.url}): super._();
  

 final  String url;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletError_ProverUrlInsecureCopyWith<WalletError_ProverUrlInsecure> get copyWith => _$WalletError_ProverUrlInsecureCopyWithImpl<WalletError_ProverUrlInsecure>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_ProverUrlInsecure&&(identical(other.url, url) || other.url == url));
}


@override
int get hashCode {
    return Object.hash(runtimeType,url);
}

@override
String toString() {
    return 'WalletError.proverUrlInsecure(url: $url)';
}


}

/// @nodoc
abstract mixin class $WalletError_ProverUrlInsecureCopyWith<$Res> implements $WalletErrorCopyWith<$Res> {
  factory $WalletError_ProverUrlInsecureCopyWith(WalletError_ProverUrlInsecure value, $Res Function(WalletError_ProverUrlInsecure) _then) = _$WalletError_ProverUrlInsecureCopyWithImpl;
@useResult
$Res call({
 String url
});




}
/// @nodoc
class _$WalletError_ProverUrlInsecureCopyWithImpl<$Res>
    implements $WalletError_ProverUrlInsecureCopyWith<$Res> {
  _$WalletError_ProverUrlInsecureCopyWithImpl(this._self, this._then);

  final WalletError_ProverUrlInsecure _self;
  final $Res Function(WalletError_ProverUrlInsecure) _then;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? url = null,}) {
  return _then(WalletError_ProverUrlInsecure(
url: null == url ? _self.url : url // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class WalletError_ProvingKeyUnknown extends WalletError {
  const WalletError_ProvingKeyUnknown({required this.name}): super._();
  

 final  String name;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletError_ProvingKeyUnknownCopyWith<WalletError_ProvingKeyUnknown> get copyWith => _$WalletError_ProvingKeyUnknownCopyWithImpl<WalletError_ProvingKeyUnknown>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_ProvingKeyUnknown&&(identical(other.name, name) || other.name == name));
}


@override
int get hashCode {
    return Object.hash(runtimeType,name);
}

@override
String toString() {
    return 'WalletError.provingKeyUnknown(name: $name)';
}


}

/// @nodoc
abstract mixin class $WalletError_ProvingKeyUnknownCopyWith<$Res> implements $WalletErrorCopyWith<$Res> {
  factory $WalletError_ProvingKeyUnknownCopyWith(WalletError_ProvingKeyUnknown value, $Res Function(WalletError_ProvingKeyUnknown) _then) = _$WalletError_ProvingKeyUnknownCopyWithImpl;
@useResult
$Res call({
 String name
});




}
/// @nodoc
class _$WalletError_ProvingKeyUnknownCopyWithImpl<$Res>
    implements $WalletError_ProvingKeyUnknownCopyWith<$Res> {
  _$WalletError_ProvingKeyUnknownCopyWithImpl(this._self, this._then);

  final WalletError_ProvingKeyUnknown _self;
  final $Res Function(WalletError_ProvingKeyUnknown) _then;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? name = null,}) {
  return _then(WalletError_ProvingKeyUnknown(
name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class WalletError_ProvingKeyMismatch extends WalletError {
  const WalletError_ProvingKeyMismatch({required this.name}): super._();
  

 final  String name;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletError_ProvingKeyMismatchCopyWith<WalletError_ProvingKeyMismatch> get copyWith => _$WalletError_ProvingKeyMismatchCopyWithImpl<WalletError_ProvingKeyMismatch>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_ProvingKeyMismatch&&(identical(other.name, name) || other.name == name));
}


@override
int get hashCode {
    return Object.hash(runtimeType,name);
}

@override
String toString() {
    return 'WalletError.provingKeyMismatch(name: $name)';
}


}

/// @nodoc
abstract mixin class $WalletError_ProvingKeyMismatchCopyWith<$Res> implements $WalletErrorCopyWith<$Res> {
  factory $WalletError_ProvingKeyMismatchCopyWith(WalletError_ProvingKeyMismatch value, $Res Function(WalletError_ProvingKeyMismatch) _then) = _$WalletError_ProvingKeyMismatchCopyWithImpl;
@useResult
$Res call({
 String name
});




}
/// @nodoc
class _$WalletError_ProvingKeyMismatchCopyWithImpl<$Res>
    implements $WalletError_ProvingKeyMismatchCopyWith<$Res> {
  _$WalletError_ProvingKeyMismatchCopyWithImpl(this._self, this._then);

  final WalletError_ProvingKeyMismatch _self;
  final $Res Function(WalletError_ProvingKeyMismatch) _then;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? name = null,}) {
  return _then(WalletError_ProvingKeyMismatch(
name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class WalletError_ProvingKeyDownloadFailed extends WalletError {
  const WalletError_ProvingKeyDownloadFailed({required this.name}): super._();
  

 final  String name;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletError_ProvingKeyDownloadFailedCopyWith<WalletError_ProvingKeyDownloadFailed> get copyWith => _$WalletError_ProvingKeyDownloadFailedCopyWithImpl<WalletError_ProvingKeyDownloadFailed>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_ProvingKeyDownloadFailed&&(identical(other.name, name) || other.name == name));
}


@override
int get hashCode {
    return Object.hash(runtimeType,name);
}

@override
String toString() {
    return 'WalletError.provingKeyDownloadFailed(name: $name)';
}


}

/// @nodoc
abstract mixin class $WalletError_ProvingKeyDownloadFailedCopyWith<$Res> implements $WalletErrorCopyWith<$Res> {
  factory $WalletError_ProvingKeyDownloadFailedCopyWith(WalletError_ProvingKeyDownloadFailed value, $Res Function(WalletError_ProvingKeyDownloadFailed) _then) = _$WalletError_ProvingKeyDownloadFailedCopyWithImpl;
@useResult
$Res call({
 String name
});




}
/// @nodoc
class _$WalletError_ProvingKeyDownloadFailedCopyWithImpl<$Res>
    implements $WalletError_ProvingKeyDownloadFailedCopyWith<$Res> {
  _$WalletError_ProvingKeyDownloadFailedCopyWithImpl(this._self, this._then);

  final WalletError_ProvingKeyDownloadFailed _self;
  final $Res Function(WalletError_ProvingKeyDownloadFailed) _then;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? name = null,}) {
  return _then(WalletError_ProvingKeyDownloadFailed(
name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class WalletError_ProvingKeyCorrupt extends WalletError {
  const WalletError_ProvingKeyCorrupt({required this.name}): super._();
  

 final  String name;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletError_ProvingKeyCorruptCopyWith<WalletError_ProvingKeyCorrupt> get copyWith => _$WalletError_ProvingKeyCorruptCopyWithImpl<WalletError_ProvingKeyCorrupt>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_ProvingKeyCorrupt&&(identical(other.name, name) || other.name == name));
}


@override
int get hashCode {
    return Object.hash(runtimeType,name);
}

@override
String toString() {
    return 'WalletError.provingKeyCorrupt(name: $name)';
}


}

/// @nodoc
abstract mixin class $WalletError_ProvingKeyCorruptCopyWith<$Res> implements $WalletErrorCopyWith<$Res> {
  factory $WalletError_ProvingKeyCorruptCopyWith(WalletError_ProvingKeyCorrupt value, $Res Function(WalletError_ProvingKeyCorrupt) _then) = _$WalletError_ProvingKeyCorruptCopyWithImpl;
@useResult
$Res call({
 String name
});




}
/// @nodoc
class _$WalletError_ProvingKeyCorruptCopyWithImpl<$Res>
    implements $WalletError_ProvingKeyCorruptCopyWith<$Res> {
  _$WalletError_ProvingKeyCorruptCopyWithImpl(this._self, this._then);

  final WalletError_ProvingKeyCorrupt _self;
  final $Res Function(WalletError_ProvingKeyCorrupt) _then;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? name = null,}) {
  return _then(WalletError_ProvingKeyCorrupt(
name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class WalletError_ProvingKeyStoreFailed extends WalletError {
  const WalletError_ProvingKeyStoreFailed({required this.path}): super._();
  

 final  String path;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletError_ProvingKeyStoreFailedCopyWith<WalletError_ProvingKeyStoreFailed> get copyWith => _$WalletError_ProvingKeyStoreFailedCopyWithImpl<WalletError_ProvingKeyStoreFailed>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_ProvingKeyStoreFailed&&(identical(other.path, path) || other.path == path));
}


@override
int get hashCode {
    return Object.hash(runtimeType,path);
}

@override
String toString() {
    return 'WalletError.provingKeyStoreFailed(path: $path)';
}


}

/// @nodoc
abstract mixin class $WalletError_ProvingKeyStoreFailedCopyWith<$Res> implements $WalletErrorCopyWith<$Res> {
  factory $WalletError_ProvingKeyStoreFailedCopyWith(WalletError_ProvingKeyStoreFailed value, $Res Function(WalletError_ProvingKeyStoreFailed) _then) = _$WalletError_ProvingKeyStoreFailedCopyWithImpl;
@useResult
$Res call({
 String path
});




}
/// @nodoc
class _$WalletError_ProvingKeyStoreFailedCopyWithImpl<$Res>
    implements $WalletError_ProvingKeyStoreFailedCopyWith<$Res> {
  _$WalletError_ProvingKeyStoreFailedCopyWithImpl(this._self, this._then);

  final WalletError_ProvingKeyStoreFailed _self;
  final $Res Function(WalletError_ProvingKeyStoreFailed) _then;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? path = null,}) {
  return _then(WalletError_ProvingKeyStoreFailed(
path: null == path ? _self.path : path // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class WalletError_PoseidonInputCountInvalid extends WalletError {
  const WalletError_PoseidonInputCountInvalid({required this.count}): super._();
  

 final  BigInt count;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletError_PoseidonInputCountInvalidCopyWith<WalletError_PoseidonInputCountInvalid> get copyWith => _$WalletError_PoseidonInputCountInvalidCopyWithImpl<WalletError_PoseidonInputCountInvalid>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_PoseidonInputCountInvalid&&(identical(other.count, count) || other.count == count));
}


@override
int get hashCode {
    return Object.hash(runtimeType,count);
}

@override
String toString() {
    return 'WalletError.poseidonInputCountInvalid(count: $count)';
}


}

/// @nodoc
abstract mixin class $WalletError_PoseidonInputCountInvalidCopyWith<$Res> implements $WalletErrorCopyWith<$Res> {
  factory $WalletError_PoseidonInputCountInvalidCopyWith(WalletError_PoseidonInputCountInvalid value, $Res Function(WalletError_PoseidonInputCountInvalid) _then) = _$WalletError_PoseidonInputCountInvalidCopyWithImpl;
@useResult
$Res call({
 BigInt count
});




}
/// @nodoc
class _$WalletError_PoseidonInputCountInvalidCopyWithImpl<$Res>
    implements $WalletError_PoseidonInputCountInvalidCopyWith<$Res> {
  _$WalletError_PoseidonInputCountInvalidCopyWithImpl(this._self, this._then);

  final WalletError_PoseidonInputCountInvalid _self;
  final $Res Function(WalletError_PoseidonInputCountInvalid) _then;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? count = null,}) {
  return _then(WalletError_PoseidonInputCountInvalid(
count: null == count ? _self.count : count // ignore: cast_nullable_to_non_nullable
as BigInt,
  ));
}


}

/// @nodoc


class WalletError_PoseidonInputLengthInvalid extends WalletError {
  const WalletError_PoseidonInputLengthInvalid({required this.index, required this.length}): super._();
  

 final  BigInt index;
 final  BigInt length;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletError_PoseidonInputLengthInvalidCopyWith<WalletError_PoseidonInputLengthInvalid> get copyWith => _$WalletError_PoseidonInputLengthInvalidCopyWithImpl<WalletError_PoseidonInputLengthInvalid>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_PoseidonInputLengthInvalid&&(identical(other.index, index) || other.index == index)&&(identical(other.length, length) || other.length == length));
}


@override
int get hashCode {
    return Object.hash(runtimeType,index,length);
}

@override
String toString() {
    return 'WalletError.poseidonInputLengthInvalid(index: $index, length: $length)';
}


}

/// @nodoc
abstract mixin class $WalletError_PoseidonInputLengthInvalidCopyWith<$Res> implements $WalletErrorCopyWith<$Res> {
  factory $WalletError_PoseidonInputLengthInvalidCopyWith(WalletError_PoseidonInputLengthInvalid value, $Res Function(WalletError_PoseidonInputLengthInvalid) _then) = _$WalletError_PoseidonInputLengthInvalidCopyWithImpl;
@useResult
$Res call({
 BigInt index, BigInt length
});




}
/// @nodoc
class _$WalletError_PoseidonInputLengthInvalidCopyWithImpl<$Res>
    implements $WalletError_PoseidonInputLengthInvalidCopyWith<$Res> {
  _$WalletError_PoseidonInputLengthInvalidCopyWithImpl(this._self, this._then);

  final WalletError_PoseidonInputLengthInvalid _self;
  final $Res Function(WalletError_PoseidonInputLengthInvalid) _then;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? index = null,Object? length = null,}) {
  return _then(WalletError_PoseidonInputLengthInvalid(
index: null == index ? _self.index : index // ignore: cast_nullable_to_non_nullable
as BigInt,length: null == length ? _self.length : length // ignore: cast_nullable_to_non_nullable
as BigInt,
  ));
}


}

/// @nodoc


class WalletError_SignerMissing extends WalletError {
  const WalletError_SignerMissing(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_SignerMissing);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'WalletError.signerMissing()';
}


}




/// @nodoc


class WalletError_SignerMismatch extends WalletError {
  const WalletError_SignerMismatch({required this.wallet, required this.signer}): super._();
  

 final  String wallet;
 final  String signer;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletError_SignerMismatchCopyWith<WalletError_SignerMismatch> get copyWith => _$WalletError_SignerMismatchCopyWithImpl<WalletError_SignerMismatch>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_SignerMismatch&&(identical(other.wallet, wallet) || other.wallet == wallet)&&(identical(other.signer, signer) || other.signer == signer));
}


@override
int get hashCode {
    return Object.hash(runtimeType,wallet,signer);
}

@override
String toString() {
    return 'WalletError.signerMismatch(wallet: $wallet, signer: $signer)';
}


}

/// @nodoc
abstract mixin class $WalletError_SignerMismatchCopyWith<$Res> implements $WalletErrorCopyWith<$Res> {
  factory $WalletError_SignerMismatchCopyWith(WalletError_SignerMismatch value, $Res Function(WalletError_SignerMismatch) _then) = _$WalletError_SignerMismatchCopyWithImpl;
@useResult
$Res call({
 String wallet, String signer
});




}
/// @nodoc
class _$WalletError_SignerMismatchCopyWithImpl<$Res>
    implements $WalletError_SignerMismatchCopyWith<$Res> {
  _$WalletError_SignerMismatchCopyWithImpl(this._self, this._then);

  final WalletError_SignerMismatch _self;
  final $Res Function(WalletError_SignerMismatch) _then;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? wallet = null,Object? signer = null,}) {
  return _then(WalletError_SignerMismatch(
wallet: null == wallet ? _self.wallet : wallet // ignore: cast_nullable_to_non_nullable
as String,signer: null == signer ? _self.signer : signer // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class WalletError_UnexpectedSigners extends WalletError {
  const WalletError_UnexpectedSigners({required  List<String> signers}): _signers = signers,super._();
  

 final  List<String> _signers;
 List<String> get signers {
  if (_signers is EqualUnmodifiableListView) return _signers;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_signers);
}


/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletError_UnexpectedSignersCopyWith<WalletError_UnexpectedSigners> get copyWith => _$WalletError_UnexpectedSignersCopyWithImpl<WalletError_UnexpectedSigners>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_UnexpectedSigners&&const DeepCollectionEquality().equals(other.signers, _signers));
}


@override
int get hashCode {
    return Object.hash(runtimeType,const DeepCollectionEquality().hash(_signers));
}

@override
String toString() {
    return 'WalletError.unexpectedSigners(signers: $signers)';
}


}

/// @nodoc
abstract mixin class $WalletError_UnexpectedSignersCopyWith<$Res> implements $WalletErrorCopyWith<$Res> {
  factory $WalletError_UnexpectedSignersCopyWith(WalletError_UnexpectedSigners value, $Res Function(WalletError_UnexpectedSigners) _then) = _$WalletError_UnexpectedSignersCopyWithImpl;
@useResult
$Res call({
 List<String> signers
});




}
/// @nodoc
class _$WalletError_UnexpectedSignersCopyWithImpl<$Res>
    implements $WalletError_UnexpectedSignersCopyWith<$Res> {
  _$WalletError_UnexpectedSignersCopyWithImpl(this._self, this._then);

  final WalletError_UnexpectedSigners _self;
  final $Res Function(WalletError_UnexpectedSigners) _then;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? signers = null,}) {
  return _then(WalletError_UnexpectedSigners(
signers: null == signers ? _self._signers : signers // ignore: cast_nullable_to_non_nullable
as List<String>,
  ));
}


}

/// @nodoc


class WalletError_WalletClosed extends WalletError {
  const WalletError_WalletClosed(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_WalletClosed);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'WalletError.walletClosed()';
}


}




/// @nodoc


class WalletError_Client extends WalletError {
  const WalletError_Client({required this.message}): super._();
  

 final  String message;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletError_ClientCopyWith<WalletError_Client> get copyWith => _$WalletError_ClientCopyWithImpl<WalletError_Client>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletError_Client&&(identical(other.message, message) || other.message == message));
}


@override
int get hashCode {
    return Object.hash(runtimeType,message);
}

@override
String toString() {
    return 'WalletError.client(message: $message)';
}


}

/// @nodoc
abstract mixin class $WalletError_ClientCopyWith<$Res> implements $WalletErrorCopyWith<$Res> {
  factory $WalletError_ClientCopyWith(WalletError_Client value, $Res Function(WalletError_Client) _then) = _$WalletError_ClientCopyWithImpl;
@useResult
$Res call({
 String message
});




}
/// @nodoc
class _$WalletError_ClientCopyWithImpl<$Res>
    implements $WalletError_ClientCopyWith<$Res> {
  _$WalletError_ClientCopyWithImpl(this._self, this._then);

  final WalletError_Client _self;
  final $Res Function(WalletError_Client) _then;

/// Create a copy of WalletError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? message = null,}) {
  return _then(WalletError_Client(
message: null == message ? _self.message : message // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

// dart format on
