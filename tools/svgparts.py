"""The shapes in a LaserMadeMusic cut file: every path, as a polygon with its
stroke colour, and the few measures a bore reader needs of it.

The generators draw only M, L, A and Z (and the odd H/V), in absolute
millimetre coordinates, with every path inside a <g stroke=...> of one
colour. That is all this reads; anything else is an error rather than a guess.
"""
import math
import re
import xml.etree.ElementTree as ET

NS = '{http://www.w3.org/2000/svg}'
TOKEN = re.compile(r'[MLHVAZmlhvaz]|-?\d*\.?\d+(?:e-?\d+)?')


class Shape:
    def __init__(self, colour, pts, closed):
        self.colour, self.pts, self.closed = colour, pts, closed

    @property
    def area(self):
        p = self.pts
        return abs(sum(p[i][0] * p[i - 1][1] - p[i - 1][0] * p[i][1]
                       for i in range(len(p)))) / 2

    @property
    def perimeter(self):
        p = self.pts
        n = len(p) if self.closed else len(p) - 1
        return sum(math.dist(p[i], p[(i + 1) % len(p)]) for i in range(n))

    @property
    def centre(self):
        xs, ys = [q[0] for q in self.pts], [q[1] for q in self.pts]
        return (min(xs) + max(xs)) / 2, (min(ys) + max(ys)) / 2

    @property
    def bbox(self):
        xs, ys = [q[0] for q in self.pts], [q[1] for q in self.pts]
        return min(xs), min(ys), max(xs), max(ys)

    def cut_area(self, kerf):
        """The area once cut, for a HOLE drawn kerf/2 inside its line: the beam
        takes kerf/2 more all round."""
        h = kerf / 2
        return self.area + self.perimeter * h + math.pi * h * h

    def inside(self, x, y):
        p, n, c = self.pts, len(self.pts), False
        for i in range(n):
            (x1, y1), (x2, y2) = p[i], p[i - 1]
            if (y1 > y) != (y2 > y) and x < x1 + (y - y1) * (x2 - x1) / (y2 - y1):
                c = not c
        return c


class Circle(Shape):
    """A <circle>: its measures exact, its points a 72-gon for inside()."""

    def __init__(self, colour, x, y, r):
        pts = [(x + r * math.cos(2 * math.pi * i / 72), y + r * math.sin(2 * math.pi * i / 72))
               for i in range(72)]
        super().__init__(colour, pts, True)
        self.x, self.y, self.r = x, y, r

    @property
    def area(self):
        return math.pi * self.r ** 2

    @property
    def perimeter(self):
        return 2 * math.pi * self.r

    @property
    def centre(self):
        return self.x, self.y


def arc(p0, rx, ry, rot, large, sweep, p1, step=math.radians(2)):
    """An SVG elliptical arc as points, the end included (SVG 1.1 F.6.5)."""
    if rx == 0 or ry == 0:
        return [p1]
    phi = math.radians(rot)
    cp, sp = math.cos(phi), math.sin(phi)
    dx, dy = (p0[0] - p1[0]) / 2, (p0[1] - p1[1]) / 2
    x1, y1 = cp * dx + sp * dy, -sp * dx + cp * dy
    lam = x1 * x1 / (rx * rx) + y1 * y1 / (ry * ry)
    if lam > 1:
        rx, ry = rx * math.sqrt(lam), ry * math.sqrt(lam)
    num = rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1
    co = math.sqrt(max(0.0, num / (rx * rx * y1 * y1 + ry * ry * x1 * x1)))
    if large == sweep:
        co = -co
    cx1, cy1 = co * rx * y1 / ry, -co * ry * x1 / rx
    cx_ = cp * cx1 - sp * cy1 + (p0[0] + p1[0]) / 2
    cy_ = sp * cx1 + cp * cy1 + (p0[1] + p1[1]) / 2
    ang = lambda ux, uy, vx, vy: math.atan2(ux * vy - uy * vx, ux * vx + uy * vy)
    t1 = ang(1, 0, (x1 - cx1) / rx, (y1 - cy1) / ry)
    dt = ang((x1 - cx1) / rx, (y1 - cy1) / ry, (-x1 - cx1) / rx, (-y1 - cy1) / ry)
    if not sweep and dt > 0:
        dt -= 2 * math.pi
    elif sweep and dt < 0:
        dt += 2 * math.pi
    n = max(2, int(abs(dt) / step) + 1)
    out = []
    for k in range(1, n + 1):
        t = t1 + dt * k / n
        x, y = rx * math.cos(t), ry * math.sin(t)
        out.append((cp * x - sp * y + cx_, sp * x + cp * y + cy_))
    out[-1] = p1
    return out


def parse_d(d):
    out, cur, pos, cmd = [], None, (0.0, 0.0), None
    toks = TOKEN.findall(d)
    i = 0
    while i < len(toks):
        t = toks[i]
        if t in 'MLHVAZ':
            cmd = t
            i += 1
            if t == 'Z':
                if cur:
                    out.append((cur, True))
                cur = None
            continue
        if t in 'mlhvaz':
            raise ValueError(f'relative path command {t}: not read')
        if cmd == 'M':
            if cur:
                out.append((cur, False))
            pos = (float(toks[i]), float(toks[i + 1]))
            cur = [pos]
            i += 2
            cmd = 'L'
        elif cmd == 'L':
            pos = (float(toks[i]), float(toks[i + 1]))
            cur.append(pos)
            i += 2
        elif cmd == 'A':
            rx, ry, rot, large, sweep, x, y = (float(v) for v in toks[i:i + 7])
            cur.extend(arc(pos, rx, ry, rot, large, sweep, (x, y)))
            pos = (x, y)
            i += 7
        elif cmd == 'H':
            pos = (float(t), pos[1])
            cur.append(pos)
            i += 1
        elif cmd == 'V':
            pos = (pos[0], float(t))
            cur.append(pos)
            i += 1
        else:
            raise ValueError(f'unexpected {t}')
    if cur:
        out.append((cur, False))
    return out


def shapes(path):
    """Every shape in the file, with its colour and the file's <desc>."""
    root = ET.parse(path).getroot()
    desc = root.findtext(NS + 'desc') or ''
    found = []

    def walk(el, colour):
        colour = el.get('stroke', colour)
        if el.tag == NS + 'path':
            for pts, closed in parse_d(el.get('d')):
                if closed and len(pts) > 1 and pts[0] == pts[-1]:
                    pts = pts[:-1]
                found.append(Shape(colour, pts, closed))
        elif el.tag == NS + 'circle':
            cx_, cy_, r = (float(el.get(k)) for k in ('cx', 'cy', 'r'))
            found.append(Circle(colour, cx_, cy_, r))
        elif el.tag in (NS + 'ellipse', NS + 'rect', NS + 'polygon',
                        NS + 'polyline', NS + 'line'):
            raise ValueError(f'{path}: a <{el.tag[len(NS):]}>, which this does not read')
        for c in el:
            walk(c, colour)
    walk(root, None)
    return found, desc


def kerf_of(desc):
    m = re.search(r'drawn for a ([\d.]+)mm kerf', desc)
    if not m:
        raise ValueError('the file does not say what kerf it is drawn for')
    return float(m.group(1))


def eq_d(area):
    return math.sqrt(4 * area / math.pi)
