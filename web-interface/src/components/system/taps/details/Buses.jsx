import React from 'react'
import Bus from './Bus'

function Buses (props) {
  if (!props.tap.active) {
    return <div className="alert alert-warning">No recent data.</div>
  }

  return (
        <div>
            {Object.keys(props.tap.buses).map(function (key, i) {
              return <Bus key={'bus-' + i} bus={props.tap.buses[i]} tap={props.tap} />
            })}
        </div>
  )
}

export default Buses
