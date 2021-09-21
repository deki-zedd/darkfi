q = 0x40000000000000000000000000000000224698fc0994a8dd8c46eb2100000001
K = GF(q)
a = K(0x00)
b = K(0x05)
E = EllipticCurve(K, (a, b))
G = E(0x40000000000000000000000000000000224698fc0994a8dd8c46eb2100000000, 0x02)

p = 0x40000000000000000000000000000000224698fc094cf91b992d30ed00000001
assert E.order() == p
F = GF(p)

Poly.<X> = F[]

k = 3
n = 2^k

x = F(88)

px = (F(110) + F(56) * X + F(89) * X^2 + F(6543) * X^3
      + F(2) * X^4 + F(110) * X^5 + F(44) * X^6 + F(78) * X^7)
assert px.degree() <= n

base_G = [E.random_element(), E.random_element(), E.random_element(),
          E.random_element(), E.random_element(), E.random_element(),
          E.random_element(), E.random_element()]
base_H = E.random_element()
base_U = E.random_element()

# Make the initial commitment to px
blind = F.random_element()
C = int(blind) * base_H + sum(int(k) * G for k, G in zip(px, base_G))

# Dot product
def dot(x, y):
    result = None
    for x_i, y_i in zip(x, y):
        if result is None:
            result = int(x_i) * y_i
        else:
            result += int(x_i) * y_i
    return result

## Step 2
# Sample a random polynomial of degree n - 1
s_poly = Poly([F.random_element() for _ in range(n)])
# Polynomial should evaluate to 0 at x
s_poly -= s_poly(x)
assert s_poly(x) == 0

## Step 3
# Commitment randomness
s_poly_blind = F.random_element()

## Step 4
s_poly_commitment = (int(s_poly_blind) * base_H
                     + sum(int(k) * G for k, G in zip(s_poly, base_G)))

## Step 5
iota = F.random_element()

## Step 8 (following Halo2 not BCSM20 order)
z = F.random_element()

## Step 6
final_poly = s_poly * iota + px
##############################
# This code is not in BCSM20 #
##############################
final_poly -= final_poly(x)
assert final_poly(x) == 0
##############################

## Step 7
blind = s_poly_blind * iota + blind

# Step 8 creation of C' does not happen in Halo2 (see the notes
# from "Comparison to other work")

# Initialize the vectors in step 8
a = list(final_poly)
assert len(a) == n

b = [x^i for i in range(n)]
assert len(b) == len(a)
assert dot(a, b) == final_poly(x)

# Now loop from 3, 2, 1
half_3 = 2^2
assert half_3 * 2 == len(a) == len(b) == len(base_G)

a_lo_4, a_hi_4 = a[:half_3], a[half_3:]
b_lo_4, b_hi_4 = b[:half_3], b[half_3:]
G_lo_4, G_hi_4 = base_G[:half_3], base_G[half_3:]

l_3 = dot(a_hi_4, G_lo_4)
r_3 = dot(a_lo_4, G_hi_4)
value_l_3 = dot(a_hi_4, b_lo_4)
value_r_3 = dot(a_lo_4, b_hi_4)
l_randomness_3 = F.random_element()
r_randomness_3 = F.random_element()
l_3 += (int(value_l_3 * z) * base_U
        + int(l_randomness_3) * base_H)
r_3 += (int(value_r_3 * z) * base_U
        + int(r_randomness_3) * base_H)

challenge_3 = F.random_element()

a_3 = [a_lo_4_i + challenge_3^-1 * a_hi_4_i
       for a_lo_4_i, a_hi_4_i in zip(a_lo_4, a_hi_4)]
b_3 = [b_lo_4_i + challenge_3 * b_hi_4_i
       for b_lo_4_i, b_hi_4_i in zip(b_lo_4, b_hi_4)]
G_3 = [G_lo_4_i + int(challenge_3) * G_hi_4_i
       for G_lo_4_i, G_hi_4_i in zip(G_lo_4, G_hi_4)]

# Not in the paper
blind += l_randomness_3 * challenge_3^-1
blind += r_randomness_3 * challenge_3

# k = 2
half_2 = 2^1
assert half_2 * 2 == len(a_3) == len(b_3) == len(G_3)

a_lo_3, a_hi_3 = a_3[:half_2], a_3[half_2:]
b_lo_3, b_hi_3 = b_3[:half_2], b_3[half_2:]
G_lo_3, G_hi_3 = G_3[:half_2], G_3[half_2:]

l_2 = dot(a_hi_3, G_lo_3)
r_2 = dot(a_lo_3, G_hi_3)
value_l_2 = dot(a_hi_3, b_lo_3)
value_r_2 = dot(a_lo_3, b_hi_3)
l_randomness_2 = F.random_element()
r_randomness_2 = F.random_element()
l_2 += (int(value_l_2 * z) * base_U
        + int(l_randomness_2) * base_H)
r_2 += (int(value_r_2 * z) * base_U
        + int(r_randomness_2) * base_H)

challenge_2 = F.random_element()

a_2 = [a_lo_3_i + challenge_2^-1 * a_hi_3_i
       for a_lo_3_i, a_hi_3_i in zip(a_lo_3, a_hi_3)]
b_2 = [b_lo_3_i + challenge_2 * b_hi_3_i
       for b_lo_3_i, b_hi_3_i in zip(b_lo_3, b_hi_3)]
G_2 = [G_lo_3_i + int(challenge_2) * G_hi_3_i
       for G_lo_3_i, G_hi_3_i in zip(G_lo_3, G_hi_3)]

blind += l_randomness_2 * challenge_2^-1
blind += r_randomness_2 * challenge_2

# k = 1
half_1 = 2^0
assert half_1 * 2 == len(a_2) == len(b_2) == len(G_2)

a_lo_2, a_hi_2 = a_2[:half_1], a_2[half_1:]
b_lo_2, b_hi_2 = b_2[:half_1], b_2[half_1:]
G_lo_2, G_hi_2 = G_2[:half_1], G_2[half_1:]

l_1 = dot(a_hi_2, G_lo_2)
r_1 = dot(a_lo_2, G_hi_2)
value_l_1 = dot(a_hi_2, b_lo_2)
value_r_1 = dot(a_lo_2, b_hi_2)
l_randomness_1 = F.random_element()
r_randomness_1 = F.random_element()
l_1 += (int(value_l_1 * z) * base_U
        + int(l_randomness_1) * base_H)
r_1 += (int(value_r_1 * z) * base_U
        + int(r_randomness_1) * base_H)

challenge_1 = F.random_element()

a_1 = [a_lo_2_i + challenge_1^-1 * a_hi_2_i
       for a_lo_2_i, a_hi_2_i in zip(a_lo_2, a_hi_2)]
b_1 = [b_lo_2_i + challenge_1 * b_hi_2_i
       for b_lo_2_i, b_hi_2_i in zip(b_lo_2, b_hi_2)]
G_1 = [G_lo_2_i + int(challenge_1) * G_hi_2_i
       for G_lo_2_i, G_hi_2_i in zip(G_lo_2, G_hi_2)]

blind += l_randomness_1 * challenge_1^-1
blind += r_randomness_1 * challenge_1

# Finished looping
assert len(a_1) == 1
a = a_1[0]

