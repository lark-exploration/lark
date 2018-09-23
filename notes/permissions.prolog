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

regionLess(R, R).

permEq(P1, P2) :- permLess(P1, P2), permLess(P2, P1).

permMin(P1, P2, P1) :- permLess(P1, P2).
permMin(P1, P2, P2) :- permLess(P2, P1).

permReprEq(shared(_), shared(_)).
permReprEq(shared(_), own).
permReprEq(own, shared(_)).
permReprEq(own, own).
permReprEq(borrow(_), borrow(_)).

% Types are defined like so:
%
% ```
% Type = Perm * Base
% Base = base(Name, Types) // class, struct reference
% Name = class(C) | struct(C) | placeholder(Y)
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
    placeHasType(Place, TypePlace), % the type of `X` is `PermX BaseX`
    applyPermType(Perm, TypePlace, TypeOut).

% placeType(X, Type). % input fact

placeType(PlaceOwner -> F, TypeOut) :-
    placeType(PlaceOwner, PermOwner * base(NameOwner, TypesOwner)),
    fieldType(NameOwner, F, Parameters * FieldType),
    subst(Parameters -> TypesOwner, FieldType, SubstitutedType),
    applyPermType(PermOwner, SubstitutedType, TypeOut).

% The `applyPermType` applies the permission `Perm` to the type
% `Type`, yielding the type `TypeOut` of the resulting value.
%
% applyPerm(Perm, Type, TypeOut).

applyPermType(Perm1, Perm2 * Base, TypeOut) :-
    permLess(Perm1, Perm2),
    applyPermBase(Perm1, Base, TypeOut).

applyPermBase(
    Perm,
    base(placeholder(Y), []),
    Perm * base(placeholder(Y), [])
).

applyPermBase(
    shared(R1),
    base(class(C), Types),
    shared(R1) * base(class(C), Types1)
) :-
    applyPermTypes(shared(R1), Types, Types1).

applyPermBase(
    shared(R1),
    base(struct(S), Types),
    own * base(struct(S), Types1)
) :-
    applyPermTypes(shared(R1), Types, Types1).

applyPermBase(
    borrow(R1),
    base(class(C), Types),
    borrow(R1) * base(class(C), Types)
).

applyPermBase(
    own,
    Base,
    own * Base
).

applyPermTypes(_, [], []).

applyPermTypes(Perm, [T | Ts], [T1 | T1s]) :-
    applyPermType(Perm, T, T1),
    applyPermTypes(Perm, Ts, T1s).

% Base equality
%
% baseEq(B1, B2) -- two bases are equivalent modulo permissions.
    
baseEq(base(Name, Types1), base(Name, Types2)) :-
    typesBaseEq(Types1, Types2).

typesBaseEq([_ * Base1 | Types1], [_ * Base2 | Types2]) :-
    baseEq(Base1, Base2),
    typesBaseEq(Types1, Types2).

typesBaseEq([], []).

% Type equality
%
% Two types are equal if they support the same fundamental operations.
% They may or may not be *syntatically* equal.

typeEq(Perm1 * Base1, Perm2 * Base2) :-
    baseEq(Base1, Base2),
    Base1 = base(Name, Types1),
    Base2 = base(Name, Types2),
    permEq(Perm1, Perm2),
    genericsEq(Perm1, Types1, Types2).
    
genericsEq(_, [], []).

genericsEq(PermOwner, [Type1 | Types1], [Type2 | Types2]) :-
    genericEq(PermOwner, Type1, Type2),
    genericsEq(PermOwner, Types1, Types2).

genericEq(PermOwner, Perm1 * base(N, Types1), Perm2 * base(N, Types2)) :-
    print((PermOwner, Perm1 * base(N, Types1), Perm2 * base(N, Types2))),
    permReprEq(Perm1, Perm2),
    print("permReprEq"),
    permMin(PermOwner, Perm1, PermMin1),
    print(("permMin", permMin1)),
    permMin(PermOwner, Perm2, PermMin2),
    print(("permMin2", permMin2)),
    permEq(PermMin1, PermMin2),
    print(("permEq", permMin1)),
    genericsEq(PermMin1, Types1, Types2).
