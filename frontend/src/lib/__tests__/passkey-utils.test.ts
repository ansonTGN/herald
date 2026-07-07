import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import {
  isWebAuthnSupported,
  base64urlToBuffer,
  bufferToBase64url,
  prepareCreationOptions,
  prepareRequestOptions,
  serializeAttestation,
  serializeAssertion,
} from '../passkey-utils'

/**
 * Helper: encode a string into an ArrayBuffer (UTF-8 view) for fixtures.
 */
function strToArrayBuffer(value: string): ArrayBuffer {
  return new TextEncoder().encode(value).buffer
}

/**
 * Helper: build a minimal fake `PublicKeyCredential` for serialisation tests.
 * `jsdom` does not implement WebAuthn, so we hand-roll the shape these helpers
 * actually touch.
 */
function mockCredential(
  options: {
    id?: string
    rawId?: ArrayBuffer
    type?: string
    clientDataJSON?: ArrayBuffer
    attestationObject?: ArrayBuffer
    authenticatorData?: ArrayBuffer
    signature?: ArrayBuffer
    userHandle?: ArrayBuffer | null
    transports?: string[]
  } = {}
): PublicKeyCredential {
  const rawId = options.rawId ?? strToArrayBuffer('raw-id')
  const id = options.id ?? 'raw-id'

  const credential = {
    id,
    rawId,
    type: options.type ?? 'public-key',
  } as PublicKeyCredential

  if (options.attestationObject !== undefined) {
    ;(credential as unknown as { response: unknown }).response = {
      clientDataJSON: options.clientDataJSON ?? strToArrayBuffer('client-data'),
      attestationObject: options.attestationObject ?? strToArrayBuffer('attestation'),
      getTransports: options.transports ? () => options.transports! : undefined,
    }
  } else {
    ;(credential as unknown as { response: unknown }).response = {
      authenticatorData: options.authenticatorData ?? strToArrayBuffer('auth-data'),
      clientDataJSON: options.clientDataJSON ?? strToArrayBuffer('client-data'),
      signature: options.signature ?? strToArrayBuffer('sig'),
      userHandle: options.userHandle ?? null,
    }
  }

  return credential
}

describe('isWebAuthnSupported', () => {
  const originalDescriptor = Object.getOwnPropertyDescriptor(window, 'PublicKeyCredential')

  afterEach(() => {
    // Restore / clean up the window property between tests.
    if (originalDescriptor) {
      Object.defineProperty(window, 'PublicKeyCredential', originalDescriptor)
    } else {
      // @ts-expect-error — intentionally delete an optional browser global.
      delete (window as { PublicKeyCredential?: unknown }).PublicKeyCredential
    }
  })

  it('returns false when window.PublicKeyCredential is absent', () => {
    // @ts-expect-error — intentionally delete an optional browser global.
    delete (window as { PublicKeyCredential?: unknown }).PublicKeyCredential
    expect(isWebAuthnSupported()).toBe(false)
  })

  it('returns true when window.PublicKeyCredential is present', () => {
    Object.defineProperty(window, 'PublicKeyCredential', {
      value: function PublicKeyCredential() {},
      configurable: true,
      writable: true,
    })
    expect(isWebAuthnSupported()).toBe(true)
  })
})

describe('bufferToBase64url input type acceptance', () => {
  it.each([
    ['ArrayBuffer', strToArrayBuffer('hello-world')],
    ['Uint8Array', new TextEncoder().encode('hello-world')],
  ])('produces identical output for %s and Uint8Array view', (_label, input) => {
    // ArrayBuffer and a Uint8Array view over the same bytes must encode the same.
    const arrayBuffer = input instanceof Uint8Array ? input.buffer : input
    const uint8 = input instanceof Uint8Array ? input : new Uint8Array(input)
    expect(bufferToBase64url(arrayBuffer)).toBe(bufferToBase64url(uint8))
  })
})

describe('base64url / buffer round-trip', () => {
  it('encodes a buffer to unpadded base64url', () => {
    // "hello" -> base64 "aGVsbG8=" -> base64url "aGVsbG8" (trailing = removed)
    expect(bufferToBase64url(strToArrayBuffer('hello'))).toBe('aGVsbG8')
  })

  it('encodes bytes that would otherwise contain + and /', () => {
    // Bytes 0xfb 0xff 0xbf -> base64 contains '/' and '+'; verify url-safety.
    const bytes = new Uint8Array([0xfb, 0xff, 0xbf])
    const encoded = bufferToBase64url(bytes)
    expect(encoded).not.toContain('+')
    expect(encoded).not.toContain('/')
    expect(encoded).not.toContain('=')
  })

  it('decodes an unpadded base64url string back to the original bytes', () => {
    const original = strToArrayBuffer('WebAuthn-challenge')
    const encoded = bufferToBase64url(original)
    const decoded = base64urlToBuffer(encoded)

    expect(new Uint8Array(decoded)).toEqual(new Uint8Array(original))
  })

  it('decodes a padded base64url string (tolerant of stray padding)', () => {
    const original = strToArrayBuffer('abc')
    const padded = bufferToBase64url(original) // already unpadded; re-add '='
    // base64url of "abc" is "YWJj"; with padding it is "YWJj"
    expect(base64urlToBuffer(padded)).toEqual(original)
    // explicitly test the '+' and '/' normalisation path too
    const swapped = padded.replace(/-/g, '_')
    expect(new Uint8Array(base64urlToBuffer(swapped))).toEqual(new Uint8Array(original))
  })

  it('round-trips arbitrary binary bytes', () => {
    const bytes = new Uint8Array(Array.from({ length: 256 }, (_, i) => i))
    const encoded = bufferToBase64url(bytes)
    const decoded = new Uint8Array(base64urlToBuffer(encoded))
    expect(Array.from(decoded)).toEqual(Array.from(bytes))
  })

  // Dedicated vector coverage: empty string, pure letters, and inputs whose
  // standard base64 form would contain the base64url-specific chars `-` / `_`.
  it.each([
    ['empty string', ''],
    ['pure ASCII letters', 'hello'],
    // 0xfb 0xff 0xbf -> base64 "/u+/", base64url "_u-_"
    ['bytes producing - and _', new Uint8Array([0xfb, 0xff, 0xbf])],
    ['standard multi-block vector', 'The quick brown fox jumps over the lazy dog'],
  ])('round-trips %s losslessly', (_label, input) => {
    const bytes = typeof input === 'string' ? strToArrayBuffer(input) : input
    const encoded = bufferToBase64url(bytes)
    const decoded = base64urlToBuffer(encoded)

    // base64urlToBuffer returns a real ArrayBuffer, never a Uint8Array view.
    expect(decoded).toBeInstanceOf(ArrayBuffer)
    // Decoded byteLength must match the input byte length exactly.
    expect(decoded.byteLength).toBe(bytes.byteLength)
    // And the bytes themselves round-trip identically.
    expect(Array.from(new Uint8Array(decoded))).toEqual(Array.from(new Uint8Array(bytes)))
  })
})

describe('prepareCreationOptions', () => {
  it('decodes challenge and user.id from base64url without mutating input', () => {
    const challengeB64 = bufferToBase64url(strToArrayBuffer('challenge'))
    const userIdB64 = bufferToBase64url(strToArrayBuffer('user-123'))
    const serverOptions = {
      publicKey: {
        challenge: challengeB64,
        rp: { name: 'Herald' },
        user: {
          id: userIdB64,
          name: 'user@example.com',
          displayName: 'User',
        },
        pubKeyCredParams: [{ type: 'public-key', alg: -7 }],
        attestation: 'none',
      },
    }

    const result = prepareCreationOptions(serverOptions)
    const publicKey = result.publicKey as PublicKeyCredentialCreationOptions

    expect(publicKey.challenge).toBeInstanceOf(ArrayBuffer)
    expect(new TextDecoder().decode(publicKey.challenge)).toBe('challenge')
    expect(publicKey.user?.id).toBeInstanceOf(ArrayBuffer)
    expect(new TextDecoder().decode(publicKey.user?.id as ArrayBuffer)).toBe('user-123')

    // input is not mutated
    expect((serverOptions.publicKey as { challenge: unknown }).challenge).toBe(challengeB64)
    expect((serverOptions.publicKey as { user: { id: unknown } }).user.id).toBe(userIdB64)
  })

  it('decodes excludeCredentials ids', () => {
    const credIdB64 = bufferToBase64url(strToArrayBuffer('existing-cred'))
    const serverOptions = {
      challenge: bufferToBase64url(strToArrayBuffer('c')),
      user: { id: bufferToBase64url(strToArrayBuffer('u')), name: 'u', displayName: 'U' },
      excludeCredentials: [{ type: 'public-key', id: credIdB64 }],
    }

    const result = prepareCreationOptions(serverOptions)
    const publicKey = result.publicKey as PublicKeyCredentialCreationOptions
    const excluded = publicKey.excludeCredentials?.[0]
    expect(excluded?.id).toBeInstanceOf(ArrayBuffer)
  })

  it('accepts a bare publicKey-less payload (unwrapped shape)', () => {
    const serverOptions = {
      challenge: bufferToBase64url(strToArrayBuffer('c')),
      user: { id: bufferToBase64url(strToArrayBuffer('u')), name: 'u', displayName: 'U' },
    }
    const result = prepareCreationOptions(serverOptions)
    expect((result.publicKey as PublicKeyCredentialCreationOptions).challenge).toBeInstanceOf(
      ArrayBuffer
    )
  })
})

describe('prepareRequestOptions', () => {
  it('decodes challenge and allowCredentials ids', () => {
    const challengeB64 = bufferToBase64url(strToArrayBuffer('assertion-challenge'))
    const credIdB64 = bufferToBase64url(strToArrayBuffer('cred-1'))
    const serverOptions = {
      publicKey: {
        challenge: challengeB64,
        allowCredentials: [{ type: 'public-key', id: credIdB64 }],
      },
    }

    const result = prepareRequestOptions(serverOptions)
    const publicKey = result.publicKey as PublicKeyCredentialRequestOptions

    expect(publicKey.challenge).toBeInstanceOf(ArrayBuffer)
    expect(publicKey.allowCredentials?.[0]?.id).toBeInstanceOf(ArrayBuffer)
  })

  it('forwards mediation when provided', () => {
    const serverOptions = { publicKey: { challenge: bufferToBase64url(strToArrayBuffer('c')) } }
    expect(prepareRequestOptions(serverOptions, 'conditional').mediation).toBe('conditional')
    expect(prepareRequestOptions(serverOptions, 'optional').mediation).toBe('optional')
    expect(prepareRequestOptions(serverOptions).mediation).toBeUndefined()
  })
})

describe('serializeAttestation', () => {
  beforeEach(() => {
    Object.defineProperty(window, 'PublicKeyCredential', {
      value: function PublicKeyCredential() {},
      configurable: true,
      writable: true,
    })
  })
  afterEach(() => {
    // @ts-expect-error — intentionally delete the stub global.
    delete (window as { PublicKeyCredential?: unknown }).PublicKeyCredential
  })

  it('encodes clientDataJSON and attestationObject to base64url', () => {
    const cred = mockCredential({
      clientDataJSON: strToArrayBuffer('client-data'),
      attestationObject: strToArrayBuffer('attestation'),
    })
    const result = serializeAttestation(cred) as {
      rawId: string
      type: string
      response: { clientDataJSON: string; attestationObject: string }
    }

    expect(result.rawId).toBe(bufferToBase64url(strToArrayBuffer('raw-id')))
    expect(result.type).toBe('public-key')
    expect(result.response.clientDataJSON).toBe(bufferToBase64url(strToArrayBuffer('client-data')))
    expect(result.response.attestationObject).toBe(
      bufferToBase64url(strToArrayBuffer('attestation'))
    )
  })

  it('includes transports when getTransports() is available', () => {
    const cred = mockCredential({
      attestationObject: strToArrayBuffer('att'),
      transports: ['internal', 'hybrid'],
    })
    const result = serializeAttestation(cred) as {
      response: { transports?: string[] }
    }
    expect(result.response.transports).toEqual(['internal', 'hybrid'])
  })
})

describe('serializeAssertion', () => {
  it('encodes authenticatorData, clientDataJSON, signature to base64url', () => {
    const cred = mockCredential({
      authenticatorData: strToArrayBuffer('auth-data'),
      clientDataJSON: strToArrayBuffer('client-data'),
      signature: strToArrayBuffer('sig'),
      userHandle: null,
    })
    const result = serializeAssertion(cred) as {
      rawId: string
      response: {
        authenticatorData: string
        clientDataJSON: string
        signature: string
        userHandle?: string
      }
    }

    expect(result.rawId).toBe(bufferToBase64url(strToArrayBuffer('raw-id')))
    expect(result.response.authenticatorData).toBe(bufferToBase64url(strToArrayBuffer('auth-data')))
    expect(result.response.clientDataJSON).toBe(bufferToBase64url(strToArrayBuffer('client-data')))
    expect(result.response.signature).toBe(bufferToBase64url(strToArrayBuffer('sig')))
    expect(result.response.userHandle).toBeUndefined()
  })

  it('includes userHandle when the authenticator returns one', () => {
    const cred = mockCredential({ userHandle: strToArrayBuffer('user-handle') })
    const result = serializeAssertion(cred) as {
      response: { userHandle?: string }
    }
    expect(result.response.userHandle).toBe(bufferToBase64url(strToArrayBuffer('user-handle')))
  })
})
