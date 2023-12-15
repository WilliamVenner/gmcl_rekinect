#include <stdint.h>
#pragma once

typedef void *(*CreateInterfaceFn)(const char *pName, int *pReturnCode);

class CLuaInterface
{
	uintptr_t padding;

public:
	void *lua;
};

class ILuaShared
{
public:
	virtual void padding00() = 0;
	virtual void *padding01() = 0;
	virtual void *padding02() = 0;
	virtual void *padding03() = 0;
	virtual void *padding04() = 0;
	virtual void *padding05() = 0;
	virtual CLuaInterface *GetLuaInterface(int type) = 0;
};

extern "C" void *ctor_lua_state(CreateInterfaceFn createInterface, uint32_t realm);