export const CLEAN_COMPILE = {
  stdout: 'Compile succeeded - 0 errors\n',
  stderr: '',
  exitCode: 0,
  success: true,
  durationMs: 1200,
}

export const CS_ERROR_OUTPUT = {
  stdout: '',
  stderr: `Assets/Scripts/PlayerController.cs(42,18): error CS0103: The name 'JumpForce' does not exist in the current context
Assets/Scripts/PlayerController.cs(85,3): warning CS0219: The variable 'unusedVar' is assigned but its value is never used`,
  exitCode: 1,
  success: false,
  durationMs: 2300,
}

export const MULTI_ERROR_OUTPUT = {
  stdout: '',
  stderr: `Assets/Scripts/A.cs(10,5): error CS1001: Unexpected symbol '{'
Assets/Scripts/B.cs(20,10): error CS1525: Expected ';' 
Assets/Scripts/C.cs(15,2): warning CS0168: The imported type is unused`,
  exitCode: 1,
  success: false,
  durationMs: 3100,
}
