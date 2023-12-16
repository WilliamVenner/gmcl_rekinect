#include "glua.hpp"
#include <stdio.h>
#include <functional>

extern "C" ILuaShared *get_lua_shared(CreateInterfaceFn createInterface)
{
	return (ILuaShared *)createInterface("LUASHARED003", NULL);
}

extern "C" void *get_lua_state(CLuaInterface *lua)
{
	return lua->lua;
}

extern "C" CLuaInterface *open_lua_state(ILuaShared *lua, unsigned char type)
{
	return lua->GetLuaInterface(type);
}

// TODO lookup these in rust instead
extern "C" void *lookup_vtable(void *virtualClass, const uintptr_t index)
{
	return (*(void ***)virtualClass)[index];
}