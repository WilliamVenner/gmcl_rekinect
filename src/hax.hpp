#include <stdint.h>
#pragma once

typedef void *(*CreateInterfaceFn)(const char *pName, int *pReturnCode);

class CLuaInterface
{
	uintptr_t padding;

public:
	void *lua;
};

class ILuaInterface : CLuaInterface
{
};

class ILuaShared
{
public:
	virtual ~ILuaShared() = 0;
	virtual void Init(void *(*)(const char *, int *), bool, void *, void *) = 0;
	virtual void Shutdown() = 0;
	virtual void DumpStats() = 0;
	virtual CLuaInterface *CreateLuaInterface(unsigned char, bool) = 0;
	virtual void CloseLuaInterface(CLuaInterface *) = 0;
	virtual CLuaInterface *GetLuaInterface(unsigned char) = 0;
};

extern "C" ILuaShared *get_lua_shared(CreateInterfaceFn createInterface);
extern "C" CLuaInterface *open_lua_interface(ILuaShared *lua, unsigned char type);
extern "C" void *get_lua_state(CLuaInterface *lua);