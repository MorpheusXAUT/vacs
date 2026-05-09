export type CustomButtonColor =
    | "clay"
    | "blush"
    | "lilac"
    | "mint"
    | "lavender"
    | "taupe"
    | "cadet"
    | "steel"
    | "umber"
    | "lagoon"
    | "snow"
    | "azure";

export const CustomButtonColors: Record<CustomButtonColor, string> = {
    clay: "bg-[#e68765] border-t-[#eb9f84] border-l-[#eb9f84] border-r-[#965842] border-b-[#965842]",
    blush: "bg-[#ebc3bc] border-t-[#f0d2cd] border-l-[#f0d2cd] border-r-[#584947] border-b-[#584947]",
    lilac: "bg-[#db9acc] border-t-[#e4b3d9] border-l-[#e4b3d9] border-r-[#523a4d] border-b-[#523a4d]",
    mint: "bg-[#abdecc] border-t-[#c0e6d9] border-l-[#c0e6d9] border-r-[#40544d] border-b-[#40544d]",
    lavender:
        "bg-[#b9abde] border-t-[#cbc0e6] border-l-[#cbc0e6] border-r-[#464054] border-b-[#464054]",
    taupe: "bg-[#bba58f] border-t-[#ccbcab] border-l-[#ccbcab] border-r-[#463e36] border-b-[#463e36]",
    cadet: "bg-[#8ca1d1] border-t-[#a9b9dd] border-l-[#a9b9dd] border-r-[#353d4f] border-b-[#353d4f]",
    steel: "bg-[#8fa6b4] border-t-[#abbcc7] border-l-[#abbcc7] border-r-[#363f44] border-b-[#363f44]",
    umber: "bg-[#a98874] border-t-[#bca391] border-l-[#bca391] border-r-[#3f332b] border-b-[#3f332b]",
    lagoon: "bg-[#73b7c2] border-t-[#95cad1] border-l-[#95cad1] border-r-[#2d4649] border-b-[#2d4649]",
    snow: "bg-[#f9f9f9] border-t-white border-l-white border-r-[#606060] border-b-[#606060]",
    azure: "bg-[#89addc] border-t-[#a7c2e5] border-l-[#a7c2e5] border-r-[#344153] border-b-[#344153]",
};

export const CustomActiveButtonColors: Record<CustomButtonColor, string> = {
    clay: "active:border-r-[#eb9f84] active:border-b-[#eb9f84] active:border-t-[#965842] active:border-l-[#965842]",
    blush: "active:border-r-[#f0d2cd] active:border-b-[#f0d2cd] active:border-t-[#584947] active:border-l-[#584947]",
    lilac: "active:border-r-[#e4b3d9] active:border-b-[#e4b3d9] active:border-t-[#523a4d] active:border-l-[#523a4d]",
    mint: "active:border-r-[#c0e6d9] active:border-b-[#c0e6d9] active:border-t-[#40544d] active:border-l-[#40544d]",
    lavender:
        "active:border-r-[#cbc0e6] active:border-b-[#cbc0e6] active:border-t-[#464054] active:border-l-[#464054]",
    taupe: "active:border-r-[#ccbcab] active:border-b-[#ccbcab] active:border-t-[#463e36] active:border-l-[#463e36]",
    cadet: "active:border-r-[#a9b9dd] active:border-b-[#a9b9dd] active:border-t-[#353d4f] active:border-l-[#353d4f]",
    steel: "active:border-r-[#abbcc7] active:border-b-[#abbcc7] active:border-t-[#363f44] active:border-l-[#363f44]",
    umber: "active:border-r-[#bca391] active:border-b-[#bca391] active:border-t-[#3f332b] active:border-l-[#3f332b]",
    lagoon: "active:border-r-[#95cad1] active:border-b-[#95cad1] active:border-t-[#2d4649] active:border-l-[#2d4649]",
    snow: "active:border-r-white active:border-b-white active:border-t-[#606060] active:border-l-[#606060]",
    azure: "active:border-r-[#a7c2e5] active:border-b-[#a7c2e5] active:border-t-[#344153] active:border-l-[#344153]",
};

export const CustomForceDisabledButtonColors: Record<CustomButtonColor, string> = {
    clay: "border-[#b86c51]! border!",
    blush: "border-[#b0928d]! border!",
    lilac: "border-[#a47499]! border!",
    mint: "border-[#80a799]! border!",
    lavender: "border-[#8b80a7]! border!",
    taupe: "border-[#8c7c6b]! border!",
    cadet: "border-[#69799d]! border!",
    steel: "border-[#6b7d87]! border!",
    umber: "border-[#7e6655]! border!",
    lagoon: "border-[#598b92]! border!",
    snow: "border-[#858585]! border!",
    azure: "border-[#6782a5]! border!",
};

export const CustomButtonHighlightColors: Record<CustomButtonColor, string> = {
    clay: "bg-[#e68765]",
    blush: "bg-[#ebc3bc]",
    lilac: "bg-[#db9acc]",
    mint: "bg-[#abdecc]",
    lavender: "bg-[#b9abde]",
    taupe: "bg-[#bba58f]",
    cadet: "bg-[#8ca1d1]",
    steel: "bg-[#8fa6b4]",
    umber: "bg-[#a98874]",
    lagoon: "bg-[#73b7c2]",
    snow: "bg-[#f9f9f9]",
    azure: "bg-[#82caff]",
};
