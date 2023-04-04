;; This contract:
;; 1) Reads signature, message and public key from the input
;; 2) Calls sr25519_verify
;; 3) Traps if the signature is invalid

(module
    ;; import the host functions from the seal0 module
	(import "seal0" "seal_sr25519_verify" (func $seal_sr25519_verify (param i32 i32 i32 i32) (result i32)))
	(import "seal0" "seal_input" (func $seal_input (param i32 i32)))
	(import "seal0" "seal_return" (func $seal_return (param i32 i32 i32)))

	;; give the program 1 page of memory
	(import "env" "memory" (memory 1 1))

	;; [4, 8) len of signature + message + public key - 64 + 12 + 32 = 108 bytes
	;; write the length of the input (108 bytes) at offset 4
	(data (i32.const 4) "\6c")

	(func (export "deploy"))

	(func (export "call")
		;; define local variables
		(local $signature_ptr i32)
		(local $pub_key_ptr i32)
		(local $message_len i32)
		(local $message_ptr i32)

		;; set the pointers to the memory locations
		;; Memory layout during `call`
		;; [10, 74) signature
		;; [74, 106) public key
		;; [106, 118) message (12 bytes)
		(local.set $signature_ptr (i32.const 10))
		(local.set $pub_key_ptr (i32.const 74))
		(local.set $message_ptr (i32.const 106))

		;; store the input into the memory, starting at the signature and 
		;; up to 108 bytes stored at offset 4
		(call $seal_input (local.get $signature_ptr) (i32.const 4))

		;; call sr25519_verify and store the return code
		(i32.store
			(i32.const 4)
			(call $seal_sr25519_verify
				(local.get $signature_ptr)
				(local.get $pub_key_ptr)
				(i32.const 12)
				(local.get $message_ptr)
			)
		)

		;; exit with success and take transfer return code to the output buffer
		(call $seal_return (i32.const 0) (i32.const 4) (i32.const 4))
	)
)

