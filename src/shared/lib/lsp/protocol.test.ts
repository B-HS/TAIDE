import { describe, expect, test } from 'bun:test'
import {
    JSON_RPC_ERROR_CODE,
    SYMBOL_KIND,
    SYMBOL_KIND_VALUE_SET,
    createRequestIdGenerator,
    isCapabilityEnabled,
    isJsonRpcErrorResponse,
    isJsonRpcNotification,
    isJsonRpcRequest,
    isJsonRpcResponse,
    markupContentToString,
} from '@shared/lib/lsp/protocol'

describe('createRequestIdGenerator', () => {
    test('호출할 때마다 1부터 증가하는 id 를 발급한다', () => {
        const nextId = createRequestIdGenerator()
        expect(nextId()).toBe(1)
        expect(nextId()).toBe(2)
        expect(nextId()).toBe(3)
    })

    test('제너레이터마다 독립적인 카운터를 가진다', () => {
        const first = createRequestIdGenerator()
        const second = createRequestIdGenerator()
        expect(first()).toBe(1)
        expect(first()).toBe(2)
        expect(second()).toBe(1)
    })
})

describe('isJsonRpcResponse', () => {
    test('result 를 가진 메시지를 응답으로 인식한다', () => {
        expect(isJsonRpcResponse({ jsonrpc: '2.0', id: 1, result: {} })).toBe(true)
    })

    test('error 를 가진 메시지를 응답으로 인식한다', () => {
        expect(isJsonRpcResponse({ jsonrpc: '2.0', id: 1, error: { code: -1, message: 'x' } })).toBe(true)
    })

    test('id 가 없는 메시지는 응답이 아니다', () => {
        expect(isJsonRpcResponse({ jsonrpc: '2.0', method: 'foo', params: {} })).toBe(false)
    })

    test('원시값은 응답이 아니다', () => {
        expect(isJsonRpcResponse(null)).toBe(false)
        expect(isJsonRpcResponse('string')).toBe(false)
    })
})

describe('isJsonRpcErrorResponse', () => {
    test('error 필드 존재 여부로 판별한다', () => {
        expect(isJsonRpcErrorResponse({ jsonrpc: '2.0', id: 1, error: { code: -1, message: 'x' } })).toBe(true)
        expect(isJsonRpcErrorResponse({ jsonrpc: '2.0', id: 1, result: {} })).toBe(false)
    })
})

describe('isJsonRpcNotification', () => {
    test('id 없이 method 만 있으면 알림으로 인식한다', () => {
        expect(isJsonRpcNotification({ jsonrpc: '2.0', method: 'window/logMessage', params: {} })).toBe(true)
    })

    test('id 가 있으면 알림이 아니다', () => {
        expect(isJsonRpcNotification({ jsonrpc: '2.0', id: 1, method: 'initialize', params: {} })).toBe(false)
    })
})

describe('isJsonRpcRequest', () => {
    test('id 와 method 를 모두 가지면 요청으로 인식한다', () => {
        expect(isJsonRpcRequest({ jsonrpc: '2.0', id: 1, method: 'initialize', params: {} })).toBe(true)
    })

    test('id 가 없으면 요청이 아니다', () => {
        expect(isJsonRpcRequest({ jsonrpc: '2.0', method: 'window/logMessage', params: {} })).toBe(false)
    })
})

describe('JSON_RPC_ERROR_CODE', () => {
    test('MethodNotFound 는 JSON-RPC 표준값 -32601 이다', () => {
        expect(JSON_RPC_ERROR_CODE.METHOD_NOT_FOUND).toBe(-32601)
    })

    test('InternalError 는 JSON-RPC 표준값 -32603 이다', () => {
        expect(JSON_RPC_ERROR_CODE.INTERNAL_ERROR).toBe(-32603)
    })
})

describe('isCapabilityEnabled', () => {
    test('undefined 와 false 는 비활성이다', () => {
        expect(isCapabilityEnabled(undefined)).toBe(false)
        expect(isCapabilityEnabled(false)).toBe(false)
    })

    test('true 와 객체는 활성이다', () => {
        expect(isCapabilityEnabled(true)).toBe(true)
        expect(isCapabilityEnabled({})).toBe(true)
        expect(isCapabilityEnabled({ prepareProvider: true })).toBe(true)
    })
})

describe('SYMBOL_KIND_VALUE_SET', () => {
    test('LSP SymbolKind 스펙 전체 범위(1~26)를 오름차순 중복 없이 담는다', () => {
        expect(SYMBOL_KIND_VALUE_SET).toHaveLength(26)
        expect(SYMBOL_KIND_VALUE_SET).toEqual(SYMBOL_KIND_VALUE_SET.toSorted((a, b) => a - b))
        expect(new Set(SYMBOL_KIND_VALUE_SET).size).toBe(SYMBOL_KIND_VALUE_SET.length)
        expect(Math.min(...SYMBOL_KIND_VALUE_SET)).toBe(1)
        expect(Math.max(...SYMBOL_KIND_VALUE_SET)).toBe(26)
    })

    test('SYMBOL_KIND 의 값에서 유도된다', () => {
        expect(SYMBOL_KIND_VALUE_SET).toEqual(Object.values(SYMBOL_KIND))
    })
})

describe('markupContentToString', () => {
    test('문자열은 그대로 반환한다', () => {
        expect(markupContentToString('plain text')).toBe('plain text')
    })

    test('MarkupContent 는 value 필드를 반환한다', () => {
        expect(markupContentToString({ kind: 'markdown', value: '**bold**' })).toBe('**bold**')
    })

    test('undefined 는 undefined 를 반환한다', () => {
        expect(markupContentToString(undefined)).toBeUndefined()
    })
})
