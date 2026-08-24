" Vim syntax file
" Language: IOk
" Maintainer: Language Support

if exists("b:current_syntax")
  finish
endif

" Keywords
syntax keyword iokKeyword let fn struct ret if els elsif while for match break continue import as
syntax keyword iokBoolean true false
syntax keyword iokConstant null

" Special identifiers
syntax keyword iokSpecial self

" Builtin functions
syntax keyword iokBuiltin write exit chr readline eval

" Delimiters and operators
syntax match iokOperator "=>"
syntax match iokOperator "->"
syntax match iokOperator "\.\."
syntax match iokOperator "[+\-*/%~=!<>&|^]"

" Comments (after operators so // wins over single /)
syntax match iokComment "//.*$" contains=iokTodo
syntax keyword iokTodo TODO FIXME XXX contained

" Numbers
syntax match iokNumber "\<\d\+\(\.\d\+\)\?\>"

" Strings
syntax region iokString start=/"/ skip=/\\"/ end=/"/ contains=iokInterpolation,iokEscape
syntax match iokEscape "\\\([nrt\"\\]\)" contained
syntax region iokInterpolation start="{" end="}" contained contains=iokIdentifier,iokNumber,iokString

" Namespace / Module separator
syntax match iokModule "\<\a\w*\ze::"
syntax match iokScope "::"

" Types (PascalCase words)
syntax match iokType "\<[A-Z]\w*\>"

" Function calls
syntax match iokFunction "\<\h\w*\ze\s*("

" Highlight mapping
highlight default link iokKeyword Keyword
highlight default link iokBoolean Boolean
highlight default link iokConstant Constant
highlight default link iokSpecial Special
highlight default link iokBuiltin Function
highlight default link iokComment Comment
highlight default link iokTodo Todo
highlight default link iokNumber Number
highlight default link iokString String
highlight default link iokEscape SpecialChar
highlight default link iokInterpolation PreProc
highlight default link iokModule Type
highlight default link iokScope Operator
highlight default link iokOperator Operator
highlight default link iokType Type
highlight default link iokFunction Function

let b:current_syntax = "iok"
