#include <stdint.h>
#pragma once

// TODO https://www.unknowncheats.me/forum/garry-s-mod/475195-cusercmd-class.html

typedef void *(*CreateInterfaceFn)(const char *pName, int *pReturnCode);

class CLuaInterface
{
	uintptr_t padding;

public:
	void *lua;
};

class ILuaInterface : CLuaInterface
{
public:
	virtual bool Init(void *, bool) = 0;
	virtual void Shutdown() = 0;
	virtual void Cycle() = 0;
	virtual void *Global() = 0;
	virtual void *GetObject(int index) = 0;
	virtual void PushLuaObject(void *obj) = 0;
	virtual void *padding() = 0;
	virtual void LuaError(const char *err, int index) = 0;
	virtual void TypeError(const char *name, int index) = 0;
	virtual void CallInternal(int args, int rets) = 0;
	virtual void CallInternalNoReturns(int args) = 0;
	virtual bool CallInternalGetBool(int args) = 0;
	virtual const char *CallInternalGetString(int args) = 0;
	virtual bool CallInternalGet(int args, void *obj) = 0;
	virtual void NewGlobalTable(const char *name) = 0;
	virtual void *NewTemporaryObject() = 0;
	virtual bool isUserData(int index) = 0;
	virtual void *GetMetaTableObject(const char *name, int type) = 0;
	virtual void *GetMetaTableObject(int index) = 0;
	virtual void *GetReturn(int index) = 0;
	virtual bool IsServer() = 0;
	virtual bool IsClient() = 0;
	virtual bool IsMenu() = 0;
	virtual void DestroyObject(void *obj) = 0;
	virtual void *CreateObject() = 0;
	virtual void SetMember(void *table, void *key, void *value) = 0;
	virtual void GetNewTable() = 0;
	virtual void SetMember(void *table, float key) = 0;
	virtual void SetMember(void *table, float key, void *value) = 0;
	virtual void SetMember(void *table, const char *key) = 0;
	virtual void SetMember(void *table, const char *key, void *value) = 0;
	virtual void SetType(unsigned char) = 0;
	virtual void PushLong(long num) = 0;
	virtual int GetFlags(int index) = 0;
	virtual bool FindOnObjectsMetaTable(int objIndex, int keyIndex) = 0;
	virtual bool FindObjectOnTable(int tableIndex, int keyIndex) = 0;
	virtual void SetMemberFast(void *table, int keyIndex, int valueIndex) = 0;
	virtual bool RunString(const char *filename, const char *path, const char *stringToRun, bool run, bool showErrors) = 0;
	virtual bool IsEqual(void *objA, void *objB) = 0;
	virtual void Error(const char *err) = 0;
	virtual const char *GetStringOrError(int index) = 0;
	virtual bool RunLuaModule(const char *name) = 0;
	virtual bool FindAndRunScript(const char *filename, bool run, bool showErrors, const char *stringToRun, bool noReturns) = 0;
	virtual void SetPathID(const char *pathID) = 0;
	virtual const char *GetPathID() = 0;
	virtual void ErrorNoHalt(const char *fmt, ...) = 0;
	virtual void Msg(const char *fmt, ...) = 0;
	virtual void PushPath(const char *path) = 0;
	virtual void PopPath() = 0;
	virtual const char *GetPath() = 0;
	virtual int GetColor(int index) = 0;
	virtual void padding0() = 0;
	virtual int GetStack(int level, void *dbg) = 0;
	virtual int GetInfo(const char *what, void *dbg) = 0;
	virtual const char *GetLocal(void *dbg, int n) = 0;
	virtual const char *GetUpvalue(int funcIndex, int n) = 0;
	virtual bool RunStringEx(const char *filename, const char *path, const char *stringToRun, bool run, bool printErrors, bool dontPushErrors, bool noReturns) = 0;
	virtual size_t GetDataString(int index, const char **str) = 0;
	virtual void ErrorFromLua(const char *fmt, ...) = 0;
	virtual const char *GetCurrentLocation() = 0;
	virtual void padding1() = 0;
	virtual void padding2() = 0;
	virtual void padding3() = 0;
	virtual bool CallFunctionProtected(int, int, bool) = 0;
	virtual void Require(const char *name) = 0;
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
extern "C" CLuaInterface *open_lua_state(ILuaShared *lua, unsigned char type);
extern "C" void *get_lua_state(CLuaInterface *lua);
extern "C" void *lookup_vtable(void *virtualClass, const uintptr_t index);