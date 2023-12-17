#include "hax.hpp"
#include <stdio.h>
#include <functional>

extern "C" ILuaShared *get_lua_shared(CreateInterfaceFn createInterface)
{
	return (ILuaShared *)createInterface("LUASHARED003", NULL);
}

extern "C" CLuaInterface *open_lua_interface(ILuaShared *lua, unsigned char type)
{
	return lua->GetLuaInterface(type);
}

extern "C" void *get_lua_state(CLuaInterface *lua)
{
	return lua->lua;
}