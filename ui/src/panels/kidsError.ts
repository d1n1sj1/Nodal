/** Plain-language versions of the core's error messages, for Kids mode. Unknown messages pass through. */
const rules: [RegExp, string][] = [
  [/no ground connection|not connected to ground/i, 'Some parts are not connected back to the battery. Close the loop!'],
  [/cannot be solved/i, 'This circuit cannot work, like wires joining the two ends of a battery. Put a part in the loop.'],
  [/did not converge/i, 'The circuit got confused. Try a smaller battery or add a resistor.'],
  [/too large|million steps/i, 'That is too long to play. Try a shorter time.'],
  [/invalid value/i, 'A part has a value that cannot work. Check its numbers.'],
  [/circuit is empty/i, 'Nothing to play yet: add some parts first.'],
]

export const kidsError = (msg: string) => rules.find(([re]) => re.test(msg))?.[1] ?? msg
