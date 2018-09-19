% Permissions are:
% 
% Perm = shared(r)
%      | borrow(r)
%      | own
%      | Z
%
% Permissions can be ordered according to a `<=` relation,
% which also extends to regions `R`. The precise nature of
% regions is not defined in this set of rules, which assumes
% a functioning borrow check.

permLess(shared(R1), shared(R2)) :- regionLess(R1, R2).
permLess(shared(R1), borrow(R2)) :- regionLess(R1, R2).
permLess(borrow(R1), borrow(R2)) :- regionLess(R1, R2).
permLess(_, own).

% Types are defined like so:
%
% ```
% Type = Perm * TypeBase
% TypeBase = named(Name, Types) // class, struct reference
%          | typeParameter(Y)   // type parameter
% Name = class(C) | struct(C)
% ```
%
% Places are defined like so:
%
% ```
% Place = X | Place . F
% ```
%
% The key relation is `plcaeType(Place, Perm, Type)` -- accessing `Place`
% with the given permission `Perm` can be assigned the type
% `Type` in the given environment `Env`. Defined like so:

accessPlace(Place, Perm, TypeOut) :-
    placeHasType(Place, TypePlace), % the type of `X` is `PermX TypeBaseX`
    applyPermType(Perm, TypePlace, TypeOut).

% placeType(X, Type). % input fact

placeType(PlaceOwner -> F, TypeOut) :-
    placeType(PlaceOwner, PermOwner * named(NameOwner, TypesOwner)),
    fieldType(NameOwner, F, Parameters * FieldType),
    subst(Parameters -> TypesOwner, FieldType, SubstitutedType),
    applyPermType(PermOwner, SubstitutedType, TypeOut).

% The `applyPermType` applies the permission `Perm` to the type
% `Type`, yielding the type `TypeOut` of the resulting value.
%
% applyPerm(Perm, Type, TypeOut).

applyPermType(Perm1, Perm2 * TypeBase, TypeOut) :-
    permLess(Perm1, Perm2),
    applyPermTypeBase(Perm1, TypeBase, TypeOut).

applyPermTypeBase(
    Perm,
    typeParameter(Y),
    Perm * typeParameter(Y)
).

applyPermTypeBase(
    shared(R1),
    named(class(C), Types),
    shared(R1) * named(class(C), Types1)
) :-
    applyPermTypes(shared(R1), Types, Types1).

applyPermTypeBase(
    shared(R1),
    named(struct(S), Types),
    own * named(struct(S), Types1)
) :-
    applyPermTypes(shared(R1), Types, Types1).

applyPermTypeBase(
    borrow(R1),
    named(class(C), Types),
    borrow(R1) * named(class(C), Types)
).

applyPermTypeBase(
    own,
    TypeBase,
    own * TypeBase
).

applyPermTypes(_, [], []).

applyPermTypes(Perm, [T | Ts], [T1 | T1s]) :-
    applyPermType(Perm, T, T1),
    applyPermTypes(Perm, Ts, T1s).

% Expressions:
%
% E = Perm * Place
%   | ...
