#include "glua.hpp"
#include <stdio.h>

extern "C" void *ctor_lua_state(CreateInterfaceFn createInterface, uint32_t realm)
{
	ILuaShared *iface = (ILuaShared *)createInterface("LUASHARED003", NULL);
	CLuaInterface *cface = iface->GetLuaInterface(realm);
	if (cface)
	{
		return cface->lua;
	}
	else
	{
		return NULL;
	}
}