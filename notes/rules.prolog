% Prolog type rules for Lark.

% a Type = type(Perm, Base)
% a Base = base(Kind, Name, [Type])
% a Kind = struct | class
% a Perm = own | var(Var) ?

access(Perm1, type(PermT, BaseT), type(PermO, BaseO)) :-
    permits(PermT, Perm1),
    applyAccessPerm(Perm1, BaseT, PermO),
    applyOwnerPerm(Perm1, BaseT, BaseO).

applyAccessPerm(P, class, P).
applyAccessPerm(_, struct, own).

applyOwnerPerm(Perm1, [H | T], [H1 | T1]) :-
    applyOwnerPerm(Perm1, H, H1),
    applyOwnerPerm(Perm1, T, T1).
applyOwnerPerm(Perm1, [], []).

applyOwnerPerm(Perm1, base(Kind1, Name1, Types1), base(Kind1, Name1, Types2)) :-
    applyOwnerPerm(Perm1, Types1, Types2).

applyOwnerPerm(Perm1, type(PermT, BaseT), type(PermO, BaseO)) :-
    permitsConditionally(Perm1, PermT, PermO),
    applyOwnerPerm(Perm1, BaseT, BaseO).

% own Vec<borrow Vec<own String>>
%
% apply share:
% - `share Vec<share Vec<share String>>` is ok
% - `share Vec<borrow Vec<own String>>` is ok, which is a bit weird
% - `share Vec<borrow Vec<share String>>` is also ok, which is also weird
% - but those are "non-minimal types" we would never *infer*, do I care?
%
% apply borrow:
% - `borrow Vec<borrow Vec<own String>>` is ok
% - `borrow Vec<borrow Vec<borrow String>>` is not ok, because 
% - `borrow Vec<share Vec<share String>>` is not ok

